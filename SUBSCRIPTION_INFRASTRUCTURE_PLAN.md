# Real-time Subscription Infrastructure Plan

This document outlines the comprehensive plan for implementing real-time subscription infrastructure across both Rust and TypeScript SDKs to achieve 100% GraphQL API coverage.

## Overview

The subscription infrastructure will enable real-time communication between clients and the Circuit Breaker server using GraphQL subscriptions over WebSockets. This will unlock all 14 subscription operations and provide the foundation for real-time capabilities across the platform.

## Current Subscription Operations

### From Schema Analysis (14 total subscriptions):

| Domain | Subscription | Description | Priority |
|--------|--------------|-------------|----------|
| **Workflows** | `resourceUpdates` | Resource state changes | High |
| **Workflows** | `workflowEvents` | Workflow execution events | High |
| **Agents** | `agentExecutionStream` | Agent execution events | High |
| **LLM** | `llmStream` | Real-time LLM response streaming | High |
| **Analytics** | `costUpdates` | Real-time cost monitoring | Medium |
| **MCP** | `mcpServerStatusUpdates` | MCP server health changes | Medium |
| **MCP** | `mcpSessionEvents` | MCP session lifecycle events | Medium |
| **Subscriptions** | `resourceUpdates` | Consolidated resource updates | High |
| **Subscriptions** | `workflowEvents` | Consolidated workflow events | High |
| **Subscriptions** | `agentExecutionStream` | Consolidated agent events | High |
| **Subscriptions** | `llmStream` | Consolidated LLM streaming | High |
| **Subscriptions** | `costUpdates` | Consolidated cost updates | Medium |
| **Subscriptions** | `mcpServerStatusUpdates` | Consolidated MCP status | Medium |
| **Subscriptions** | `mcpSessionEvents` | Consolidated MCP sessions | Medium |

## Architecture Design

### 1. Core Infrastructure Components

#### WebSocket Client Foundation
```
┌─────────────────────────────────────────────────────────┐
│                 WebSocket Client Layer                   │
├─────────────────────────────────────────────────────────┤
│ • Connection Management                                 │
│ • Auto-reconnection with exponential backoff           │
│ • Ping/Pong heartbeat                                  │
│ • Message queuing during disconnection                 │
│ • Error handling and recovery                          │
└─────────────────────────────────────────────────────────┘
```

#### GraphQL Subscription Protocol
```
┌─────────────────────────────────────────────────────────┐
│              GraphQL Subscription Layer                  │
├─────────────────────────────────────────────────────────┤
│ • GraphQL-WS or GraphQL-SSE protocol support           │
│ • Subscription lifecycle (start, data, error, complete) │
│ • Operation ID management                               │
│ • Variable interpolation                                │
│ • Response parsing and validation                       │
└─────────────────────────────────────────────────────────┘
```

#### Subscription Manager
```
┌─────────────────────────────────────────────────────────┐
│                 Subscription Manager                     │
├─────────────────────────────────────────────────────────┤
│ • Multiple subscription handling                        │
│ • Event routing and filtering                           │
│ • Type-safe event handlers                              │
│ • Subscription registry and cleanup                     │
│ • Error isolation and recovery                          │
└─────────────────────────────────────────────────────────┘
```

### 2. SDK Integration Architecture

#### Rust SDK Architecture
```rust
// Core subscription infrastructure
pub struct SubscriptionClient {
    websocket: WebSocketManager,
    subscriptions: SubscriptionRegistry,
    event_handlers: EventHandlerRegistry,
}

// Domain-specific subscription clients
impl Client {
    pub fn subscriptions(&self) -> SubscriptionClient { /* */ }
}

impl WorkflowClient {
    pub fn subscribe_resource_updates(&self) -> ResourceUpdateSubscription { /* */ }
    pub fn subscribe_workflow_events(&self) -> WorkflowEventSubscription { /* */ }
}

impl LLMClient {
    pub fn subscribe_stream(&self, request_id: &str) -> LLMStreamSubscription { /* */ }
}
```

#### TypeScript SDK Architecture
```typescript
// Core subscription infrastructure
export class SubscriptionClient {
  private websocket: WebSocketManager;
  private subscriptions: SubscriptionRegistry;
  private eventHandlers: EventHandlerRegistry;
}

// Domain-specific subscription clients
export class Client {
  subscriptions(): SubscriptionClient { /* */ }
}

export class WorkflowClient {
  subscribeResourceUpdates(): ResourceUpdateSubscription { /* */ }
  subscribeWorkflowEvents(): WorkflowEventSubscription { /* */ }
}
```

## Implementation Plan

### Phase 1: Core WebSocket Infrastructure (Week 1)

#### Rust Implementation
- **WebSocket Client** (`src/subscriptions/websocket.rs`)
  - Connection management with `tokio-tungstenite`
  - Auto-reconnection with exponential backoff
  - Message queuing and buffering
  - Heartbeat/ping-pong implementation

- **GraphQL Subscription Protocol** (`src/subscriptions/protocol.rs`)
  - GraphQL-WS protocol implementation
  - Message serialization/deserialization
  - Operation lifecycle management
  - Error handling and validation

#### TypeScript Implementation
- **WebSocket Client** (`src/subscriptions/websocket.ts`)
  - Native WebSocket API wrapper
  - Connection state management
  - Auto-reconnection logic
  - Message buffering during disconnection

- **GraphQL Subscription Protocol** (`src/subscriptions/protocol.ts`)
  - GraphQL-WS protocol support
  - Type-safe message handling
  - Operation management
  - Error propagation

### Phase 2: Subscription Manager (Week 1-2)

#### Core Manager Implementation
```rust
// Rust
pub struct SubscriptionManager {
    active_subscriptions: HashMap<SubscriptionId, ActiveSubscription>,
    event_router: EventRouter,
    error_handler: ErrorHandler,
}

pub trait SubscriptionHandler<T> {
    fn on_data(&mut self, data: T) -> Result<()>;
    fn on_error(&mut self, error: SubscriptionError) -> Result<()>;
    fn on_complete(&mut self) -> Result<()>;
}
```

```typescript
// TypeScript
export class SubscriptionManager {
  private activeSubscriptions: Map<SubscriptionId, ActiveSubscription>;
  private eventRouter: EventRouter;
  private errorHandler: ErrorHandler;
}

export interface SubscriptionHandler<T> {
  onData(data: T): Promise<void>;
  onError(error: SubscriptionError): Promise<void>;
  onComplete(): Promise<void>;
}
```

### Phase 3: Domain-Specific Subscriptions (Week 2)

#### Workflow Subscriptions
```rust
// Resource Updates
pub struct ResourceUpdateSubscription {
    subscription_id: SubscriptionId,
    manager: Arc<SubscriptionManager>,
}

impl ResourceUpdateSubscription {
    pub async fn on_update<F>(&self, handler: F) -> Result<()>
    where F: Fn(ResourceGQL) + Send + Sync + 'static;
    
    pub async fn filter_by_workflow(&self, workflow_id: &str) -> Result<()>;
    pub async fn filter_by_state(&self, state: &str) -> Result<()>;
}

// Workflow Events  
pub struct WorkflowEventSubscription {
    subscription_id: SubscriptionId,
    manager: Arc<SubscriptionManager>,
}
```

#### LLM Streaming Subscriptions
```rust
pub struct LLMStreamSubscription {
    subscription_id: SubscriptionId,
    manager: Arc<SubscriptionManager>,
}

impl LLMStreamSubscription {
    pub async fn on_chunk<F>(&self, handler: F) -> Result<()>
    where F: Fn(LLMStreamChunk) + Send + Sync + 'static;
    
    pub async fn on_complete<F>(&self, handler: F) -> Result<()>
    where F: Fn(LLMResponse) + Send + Sync + 'static;
}
```

#### Analytics Subscriptions
```rust
pub struct CostUpdateSubscription {
    subscription_id: SubscriptionId,
    manager: Arc<SubscriptionManager>,
}

impl CostUpdateSubscription {
    pub async fn on_cost_update<F>(&self, handler: F) -> Result<()>
    where F: Fn(CostUpdate) + Send + Sync + 'static;
    
    pub async fn filter_by_user(&self, user_id: &str) -> Result<()>;
    pub async fn filter_by_project(&self, project_id: &str) -> Result<()>;
}
```

### Phase 4: Advanced Features (Week 3)

#### Subscription Builders
```rust
pub struct ResourceUpdateSubscriptionBuilder {
    client: Arc<SubscriptionClient>,
    resource_id: Option<String>,
    workflow_id: Option<String>,
    filters: Vec<SubscriptionFilter>,
}

impl ResourceUpdateSubscriptionBuilder {
    pub fn resource_id(mut self, id: &str) -> Self;
    pub fn workflow_id(mut self, id: &str) -> Self;
    pub fn filter_state(mut self, state: &str) -> Self;
    pub async fn subscribe(self) -> Result<ResourceUpdateSubscription>;
}
```

#### Event Filtering and Aggregation
```rust
pub enum SubscriptionFilter {
    ResourceId(String),
    WorkflowId(String),
    State(String),
    UserId(String),
    ProjectId(String),
    Custom(Box<dyn Fn(&serde_json::Value) -> bool + Send + Sync>),
}

pub struct EventAggregator {
    window_size: Duration,
    buffer: VecDeque<Event>,
}
```

#### Batch Subscription Management
```rust
pub struct BatchSubscriptionManager {
    subscriptions: Vec<Box<dyn Subscription>>,
}

impl BatchSubscriptionManager {
    pub fn add_subscription<T: Subscription + 'static>(&mut self, sub: T);
    pub async fn subscribe_all(&self) -> Result<()>;
    pub async fn unsubscribe_all(&self) -> Result<()>;
}
```

## Error Handling Strategy

### Connection Error Recovery
```rust
#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("WebSocket connection failed: {message}")]
    ConnectionFailed { message: String },
    
    #[error("Subscription {id} failed: {reason}")]
    SubscriptionFailed { id: SubscriptionId, reason: String },
    
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },
    
    #[error("Rate limit exceeded: {retry_after}")]
    RateLimitExceeded { retry_after: Duration },
}

pub struct ErrorRecoveryPolicy {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}
```

### Graceful Degradation
- Automatic fallback to polling for critical operations
- Offline queue for missed events
- Progressive reconnection with circuit breaker pattern

## Performance Optimizations

### Connection Pooling
```rust
pub struct SubscriptionConnectionPool {
    connections: Vec<Arc<WebSocketConnection>>,
    load_balancer: LoadBalancer,
    health_checker: HealthChecker,
}
```

### Message Batching
```rust
pub struct MessageBatcher {
    batch_size: usize,
    flush_interval: Duration,
    pending_messages: Vec<Message>,
}
```

### Memory Management
- Subscription cleanup on drop
- Automatic memory limit enforcement
- Event buffer size limits

## Testing Strategy

### Unit Tests
- WebSocket connection simulation
- Message protocol validation
- Error recovery scenarios
- Memory leak prevention

### Integration Tests
- End-to-end subscription flows
- Multi-client scenarios
- Network failure simulation
- Performance benchmarks

### Load Testing
- Concurrent subscription limits
- Memory usage under load
- Connection recovery performance
- Event throughput testing

## Usage Examples

### Rust SDK Usage
```rust
use circuit_breaker_sdk::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .base_url("ws://localhost:4000/graphql")?
        .build()?;

    // Subscribe to resource updates
    let resource_sub = client.workflows()
        .subscribe_resource_updates()
        .resource_id("resource_123")
        .subscribe()
        .await?;

    resource_sub.on_update(|resource| {
        println!("Resource updated: {} -> {}", resource.id, resource.state);
    }).await?;

    // Subscribe to LLM streaming
    let llm_sub = client.llm()
        .subscribe_stream("request_456")
        .subscribe()
        .await?;

    llm_sub.on_chunk(|chunk| {
        print!("{}", chunk.content);
    }).await?;

    // Multiple subscriptions
    let mut batch = BatchSubscriptionManager::new();
    batch.add_subscription(resource_sub);
    batch.add_subscription(llm_sub);
    batch.subscribe_all().await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    batch.unsubscribe_all().await?;

    Ok(())
}
```

### TypeScript SDK Usage
```typescript
import { Client } from './client.js';

async function main() {
  const client = Client.builder()
    .baseUrl('ws://localhost:4000/graphql')
    .build();

  // Subscribe to workflow events
  const workflowSub = await client.workflows()
    .subscribeWorkflowEvents()
    .workflowId('workflow_789')
    .subscribe();

  workflowSub.onEvent(async (event) => {
    console.log(`Workflow event: ${event.type} - ${event.message}`);
  });

  // Subscribe to cost updates
  const costSub = await client.analytics()
    .subscribeCostUpdates()
    .userId('user_123')
    .subscribe();

  costSub.onUpdate(async (update) => {
    console.log(`Cost update: $${update.amount} for ${update.service}`);
  });

  // Batch management
  const batchManager = new BatchSubscriptionManager();
  batchManager.addSubscription(workflowSub);
  batchManager.addSubscription(costSub);
  
  await batchManager.subscribeAll();

  // Cleanup on exit
  process.on('SIGINT', async () => {
    await batchManager.unsubscribeAll();
    process.exit(0);
  });
}
```

## Security Considerations

### Authentication
- JWT token refresh for long-lived subscriptions
- Secure WebSocket upgrade with proper headers
- API key validation for subscription access

### Authorization
- Subscription-level permission checking
- Resource-based access control
- Rate limiting per user/subscription

### Data Privacy
- Event filtering based on user permissions
- Sensitive data redaction in streams
- Audit logging for subscription access

## Monitoring and Observability

### Metrics Collection
```rust
pub struct SubscriptionMetrics {
    active_subscriptions: AtomicU64,
    messages_received: AtomicU64,
    connection_failures: AtomicU64,
    reconnection_attempts: AtomicU64,
}
```

### Health Checks
- WebSocket connection health
- Subscription responsiveness
- Memory usage monitoring
- Event processing latency

### Logging
- Structured logging for all subscription events
- Debug logging for connection issues
- Performance logging for optimization

## Delivery Timeline

### Week 1: Foundation
- [ ] WebSocket client implementation (Rust & TypeScript)
- [ ] GraphQL subscription protocol
- [ ] Basic connection management
- [ ] Unit tests for core components

### Week 2: Core Features
- [ ] Subscription manager implementation
- [ ] Domain-specific subscription clients
- [ ] Error handling and recovery
- [ ] Integration tests

### Week 3: Advanced Features
- [ ] Subscription builders and filters
- [ ] Batch subscription management
- [ ] Performance optimizations
- [ ] Load testing and benchmarks

### Week 4: Polish and Documentation
- [ ] Comprehensive documentation
- [ ] Usage examples and tutorials
- [ ] Performance tuning
- [ ] Release preparation

## Success Metrics

### Coverage Goals
- **100% Subscription Operations**: All 14 subscription operations working
- **Type Safety**: Full type safety for all subscription events
- **Error Recovery**: Robust error handling and automatic recovery
- **Performance**: <100ms latency for event delivery

### Quality Goals
- **Reliability**: 99.9% uptime for subscription connections
- **Scalability**: Support 1000+ concurrent subscriptions
- **Memory Efficiency**: <10MB memory overhead per 100 subscriptions
- **Developer Experience**: Intuitive APIs with comprehensive examples

This comprehensive plan will deliver production-ready real-time subscription infrastructure, completing the Circuit Breaker SDK's journey to 100% GraphQL API coverage while maintaining excellent developer experience and system reliability.