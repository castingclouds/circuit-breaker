/**
 * Real-time Subscription Infrastructure
 *
 * This module provides comprehensive real-time subscription capabilities for the Circuit Breaker SDK,
 * enabling WebSocket-based GraphQL subscriptions with automatic reconnection, error recovery,
 * and type-safe event handling.
 */
import { Client } from './client.js';
/**
 * Unique identifier for subscriptions
 */
export declare class SubscriptionId {
    private id;
    constructor(id?: string);
    toString(): string;
    static fromString(id: string): SubscriptionId;
    equals(other: SubscriptionId): boolean;
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
export type GraphQLWSMessage = {
    type: 'connection_init';
    payload?: any;
} | {
    type: 'connection_ack';
} | {
    type: 'start';
    id: string;
    payload: GraphQLSubscription;
} | {
    type: 'data';
    id: string;
    payload: any;
} | {
    type: 'error';
    id: string;
    payload: any;
} | {
    type: 'complete';
    id: string;
} | {
    type: 'stop';
    id: string;
} | {
    type: 'ping';
    payload?: any;
} | {
    type: 'pong';
    payload?: any;
};
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
export declare const defaultSubscriptionConfig: SubscriptionConfig;
/**
 * Subscription error types
 */
export declare class SubscriptionError extends Error {
    readonly subscriptionId?: SubscriptionId | undefined;
    readonly payload?: any | undefined;
    constructor(message: string, subscriptionId?: SubscriptionId | undefined, payload?: any | undefined);
    static connectionFailed(message: string): SubscriptionError;
    static graphqlError(subscriptionId: SubscriptionId, payload: any): SubscriptionError;
    static authenticationFailed(message: string): SubscriptionError;
    static timeout(timeout: number): SubscriptionError;
}
/**
 * Subscription metrics for monitoring
 */
export declare class SubscriptionMetrics {
    private _activeSubscriptions;
    private _messagesReceived;
    private _connectionFailures;
    private _reconnectionAttempts;
    get activeSubscriptions(): number;
    get messagesReceived(): number;
    get connectionFailures(): number;
    get reconnectionAttempts(): number;
    incrementActiveSubscriptions(): void;
    decrementActiveSubscriptions(): void;
    incrementMessagesReceived(): void;
    incrementConnectionFailures(): void;
    incrementReconnectionAttempts(): void;
}
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
/**
 * WebSocket connection wrapper with auto-reconnection
 */
export declare class WebSocketConnection {
    private url;
    private websocket?;
    private config;
    private connected;
    private reconnectAttempts;
    private messageQueue;
    private messageHandlers;
    constructor(url: string, config?: Partial<SubscriptionConfig>);
    connect(): Promise<void>;
    isConnected(): boolean;
    sendMessage(message: GraphQLWSMessage): void;
    onMessage(handler: (message: GraphQLWSMessage) => void): void;
    offMessage(handler: (message: GraphQLWSMessage) => void): void;
    close(): void;
    private sendConnectionInit;
    private flushMessageQueue;
    private handleMessage;
    private handleDisconnection;
}
/**
 * Active subscription state
 */
export declare class ActiveSubscription<T = any> {
    readonly id: SubscriptionId;
    readonly subscription: GraphQLSubscription;
    private handler;
    constructor(id: SubscriptionId, subscription: GraphQLSubscription, handler: SubscriptionHandler<T>);
    handleData(payload: any): Promise<void>;
    handleError(error: SubscriptionError): Promise<void>;
    handleComplete(): Promise<void>;
}
/**
 * Core subscription manager handling WebSocket connections and message routing
 */
export declare class SubscriptionManager {
    private client;
    private config;
    private websocket?;
    private subscriptions;
    private metrics;
    constructor(client: Client, config?: Partial<SubscriptionConfig>);
    subscribe<T>(subscription: GraphQLSubscription, handler: SubscriptionHandler<T>): Promise<SubscriptionId>;
    unsubscribe(subscriptionId: SubscriptionId): Promise<void>;
    getMetrics(): SubscriptionMetrics;
    close(): Promise<void>;
    private ensureConnection;
    private buildWebSocketUrl;
    private sendSubscriptionStart;
    private sendSubscriptionStop;
    private handleMessage;
    private handleSubscriptionData;
    private handleSubscriptionError;
    private handleSubscriptionComplete;
}
/**
 * Subscription client for real-time GraphQL subscriptions
 */
export declare class SubscriptionClient {
    private client;
    private manager;
    constructor(client: Client);
    /**
     * Subscribe to resource state changes
     */
    resourceUpdates(): ResourceUpdateSubscriptionBuilder;
    /**
     * Subscribe to workflow events
     */
    workflowEvents(): WorkflowEventSubscriptionBuilder;
    /**
     * Subscribe to agent execution events
     */
    agentExecutionStream(): AgentExecutionSubscriptionBuilder;
    /**
     * Subscribe to LLM response streaming
     */
    llmStream(requestId: string): LLMStreamSubscriptionBuilder;
    /**
     * Subscribe to real-time cost updates
     */
    costUpdates(): CostUpdateSubscriptionBuilder;
    /**
     * Subscribe to MCP server status updates
     */
    mcpServerStatusUpdates(): MCPServerStatusSubscriptionBuilder;
    /**
     * Subscribe to MCP session events
     */
    mcpSessionEvents(): MCPSessionEventSubscriptionBuilder;
    /**
     * Get subscription manager for advanced operations
     */
    getManager(): SubscriptionManager;
    /**
     * Get current metrics
     */
    getMetrics(): SubscriptionMetrics;
    /**
     * Close all subscriptions and connections
     */
    close(): Promise<void>;
}
/**
 * Builder for resource update subscriptions
 */
export declare class ResourceUpdateSubscriptionBuilder {
    private manager;
    private _resourceId?;
    private _workflowId?;
    constructor(manager: SubscriptionManager);
    resourceId(id: string): this;
    workflowId(id: string): this;
    subscribe(handler: (data: ResourceUpdateEvent) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for workflow event subscriptions
 */
export declare class WorkflowEventSubscriptionBuilder {
    private manager;
    private _workflowId?;
    constructor(manager: SubscriptionManager);
    workflowId(id: string): this;
    subscribe(handler: (data: WorkflowEvent) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for agent execution subscriptions
 */
export declare class AgentExecutionSubscriptionBuilder {
    private manager;
    private _executionId?;
    constructor(manager: SubscriptionManager);
    executionId(id: string): this;
    subscribe(handler: (data: AgentExecutionEvent) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for LLM stream subscriptions
 */
export declare class LLMStreamSubscriptionBuilder {
    private manager;
    private requestId;
    constructor(manager: SubscriptionManager, requestId: string);
    subscribe(handler: (data: LLMStreamChunk) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for cost update subscriptions
 */
export declare class CostUpdateSubscriptionBuilder {
    private manager;
    private _userId?;
    private _projectId?;
    constructor(manager: SubscriptionManager);
    userId(id: string): this;
    projectId(id: string): this;
    subscribe(handler: (data: CostUpdateEvent) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for MCP server status subscriptions
 */
export declare class MCPServerStatusSubscriptionBuilder {
    private manager;
    private _serverId?;
    constructor(manager: SubscriptionManager);
    serverId(id: string): this;
    subscribe(handler: (data: MCPServerStatusUpdate) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Builder for MCP session event subscriptions
 */
export declare class MCPSessionEventSubscriptionBuilder {
    private manager;
    private _userId?;
    private _serverId?;
    constructor(manager: SubscriptionManager);
    userId(id: string): this;
    serverId(id: string): this;
    subscribe(handler: (data: MCPSessionEvent) => Promise<void> | void, errorHandler?: (error: SubscriptionError) => Promise<void> | void, completeHandler?: () => Promise<void> | void): Promise<SubscriptionId>;
}
/**
 * Subscribe to resource updates with a simple callback
 */
export declare function subscribeResourceUpdates(client: Client, resourceId: string, handler: (resource: ResourceUpdateEvent) => void): Promise<SubscriptionId>;
/**
 * Subscribe to workflow events with a simple callback
 */
export declare function subscribeWorkflowEvents(client: Client, workflowId: string, handler: (event: WorkflowEvent) => void): Promise<SubscriptionId>;
/**
 * Subscribe to LLM streaming with a simple callback
 */
export declare function subscribeLLMStream(client: Client, requestId: string, handler: (chunk: LLMStreamChunk) => void): Promise<SubscriptionId>;
/**
 * Subscribe to cost updates with a simple callback
 */
export declare function subscribeCostUpdates(client: Client, userId: string, handler: (update: CostUpdateEvent) => void): Promise<SubscriptionId>;
//# sourceMappingURL=subscriptions.d.ts.map