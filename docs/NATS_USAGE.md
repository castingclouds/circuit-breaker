# NATS Storage Usage Guide

This guide explains how to use the NATS JetStream storage backend with Circuit Breaker for distributed, persistent workflow management.

## Overview

The NATS implementation provides:

- **Distributed Storage**: NATS JetStream for persistent, replicated workflow and token storage
- **Real-time Events**: Stream-based token transitions and workflow lifecycle events
- **Automatic Stream Management**: Workflow-specific streams created automatically
- **Enhanced GraphQL API**: NATS-specific queries and mutations for optimized operations
- **Subject Hierarchy**: Organized NATS subjects for efficient querying and filtering

## Prerequisites

1. **NATS Server with JetStream**: You need a NATS server running with JetStream enabled
2. **Rust Dependencies**: The circuit-breaker crate with NATS support

### Starting NATS Server

```bash
# Option 1: Using nats-server binary
nats-server --jetstream

# Option 2: Using Docker
docker run -p 4222:4222 -p 8222:8222 nats:latest --jetstream

# Option 3: Using Docker Compose (recommended for development)
# Create docker-compose.yml:
version: '3.8'
services:
  nats:
    image: nats:latest
    ports:
      - "4222:4222"
      - "8222:8222"
    command: "--jetstream --store_dir=/data"
    volumes:
      - nats_data:/data

volumes:
  nats_data:
```

## Basic Usage

### 1. Creating a NATS Storage Backend

```rust
use circuit_breaker::{NATSStorage, NATSStorageConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure NATS storage
    let config = NATSStorageConfig {
        nats_urls: vec!["nats://localhost:4222".to_string()],
        default_max_messages: 100_000,
        default_max_bytes: 512 * 1024 * 1024, // 512MB
        default_max_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
        consumer_timeout: Duration::from_secs(30),
        max_deliver: 3,
        connection_timeout: Duration::from_secs(10),
        reconnect_buffer_size: 4 * 1024 * 1024, // 4MB
    };

    // Create NATS storage
    let nats_storage = NATSStorage::new(config).await?;

    // Use with GraphQL schema
    let schema = create_schema_with_nats(std::sync::Arc::new(nats_storage));

    Ok(())
}
```

### 2. Starting a Server with NATS Storage

```rust
use circuit_breaker::{
    NATSStorage, NATSStorageConfig,
    create_schema_with_nats
};
use axum;
use async_graphql_axum;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create NATS storage
    let nats_storage = NATSStorage::with_default_config().await?;

    // Create GraphQL schema with NATS support
    let schema = create_schema_with_nats(std::sync::Arc::new(nats_storage));

    // Build Axum router
    let app = axum::Router::new()
        .route("/graphql", axum::routing::post(
            async_graphql_axum::graphql(schema.clone())
        ))
        .route("/graphql", axum::routing::get(
            async_graphql_axum::graphql_playground("GraphQL Playground", "/graphql")
        ));

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("GraphQL server running on http://localhost:8080/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}
```

## NATS-Enhanced GraphQL API

### Enhanced Token Operations

#### 1. Create Workflow Instance with NATS Tracking

```graphql
mutation CreateWorkflowInstance {
  createWorkflowInstance(input: {
    workflowId: "document_approval",
    initialData: {
      title: "Important Document",
      content: "Document content here...",
      priority: "high"
    },
    metadata: {
      department: "engineering",
      created_by: "user@example.com"
    },
    triggeredBy: "api_client"
  }) {
    id
    place
    natsSequence
    natsTimestamp
    natsSubject
    transitionHistory {
      fromPlace
      toPlace
      transitionId
      timestamp
      triggeredBy
      natsSequence
    }
  }
}
```

#### 2. Transition Token with NATS Event Publishing

```graphql
mutation TransitionWithNATS {
  transitionTokenWithNats(input: {
    tokenId: "123e4567-e89b-12d3-a456-426614174000",
    transitionId: "submit_for_review",
    newPlace: "review",
    triggeredBy: "document_author",
    data: {
      review_notes: "Ready for review"
    }
  }) {
    id
    place
    natsSequence
    transitionHistory {
      fromPlace
      toPlace
      transitionId
      timestamp
      triggeredBy
      natsSequence
    }
  }
}
```

#### 3. Query Tokens in Specific Places (NATS-Optimized)

```graphql
query TokensInPlace {
  tokensInPlace(workflowId: "document_approval", placeId: "review") {
    id
    place
    data
    natsSequence
    natsSubject
    transitionHistory {
      fromPlace
      toPlace
      timestamp
      triggeredBy
    }
  }
}
```

#### 4. Get Token with NATS Metadata

```graphql
query GetNATSToken {
  natsToken(id: "123e4567-e89b-12d3-a456-426614174000") {
    id
    workflowId
    place
    data
    natsSequence
    natsTimestamp
    natsSubject
    transitionHistory {
      fromPlace
      toPlace
      transitionId
      timestamp
      triggeredBy
      natsSequence
      metadata
    }
  }
}
```

## NATS Subject Hierarchy

The NATS implementation uses a structured subject hierarchy for efficient routing and filtering:

### Workflow Definitions
```
workflows.{workflow_id}.definition
```
Example: `workflows.document_approval.definition`

### Token Storage by Place
```
workflows.{workflow_id}.places.{place_id}.tokens
```
Examples:
- `workflows.document_approval.places.draft.tokens`
- `workflows.document_approval.places.review.tokens`
- `workflows.document_approval.places.approved.tokens`

### Workflow Events
```
workflows.{workflow_id}.events.transitions
workflows.{workflow_id}.events.lifecycle
```
Examples:
- `workflows.document_approval.events.transitions`
- `workflows.document_approval.events.lifecycle`

## Stream Configuration

Each workflow gets its own NATS stream with the following configuration:

- **Stream Name**: `WORKFLOW_{WORKFLOW_ID}` (uppercase)
- **Subjects**: All subjects for that workflow
- **Retention**: Interest-based (messages kept until acknowledged)
- **Storage**: File-based for persistence
- **Deduplication**: 2-minute window based on message ID
- **Limits**: Configurable message count, size, and age limits

## Real-time Event Streaming

### Subscribe to Token Events

```rust
use circuit_breaker::NATSStorage;
use futures::StreamExt;

async fn subscribe_to_events(nats_storage: &NATSStorage) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = nats_storage.subscribe_to_token_events("document_approval").await?;

    while let Some(message) = event_stream.next().await {
        let message = message?;
        println!("Received event: {}", String::from_utf8_lossy(&message.payload));
        message.ack().await?;
    }

    Ok(())
}
```

### Event Types

#### Token Creation Events
Published to: `workflows.{workflow_id}.events.lifecycle`
```json
{
  "event_type": "token_created",
  "token_id": "123e4567-e89b-12d3-a456-426614174000",
  "workflow_id": "document_approval",
  "place": "draft",
  "timestamp": "2024-01-15T10:30:00Z",
  "triggered_by": "api_client"
}
```

#### Token Transition Events
Published to: `workflows.{workflow_id}.events.transitions`
```json
{
  "event_type": "token_transitioned",
  "token_id": "123e4567-e89b-12d3-a456-426614174000",
  "workflow_id": "document_approval",
  "from_place": "draft",
  "to_place": "review",
  "transition_id": "submit_for_review",
  "timestamp": "2024-01-15T10:35:00Z",
  "triggered_by": "document_author"
}
```

## Advanced Usage

### Multi-Tenant Workflows

Use workflow IDs with tenant prefixes:

```rust
// Create tenant-specific workflow IDs
let tenant_a_workflow = format!("tenant_a::{}", workflow_id);
let tenant_b_workflow = format!("tenant_b::{}", workflow_id);

// Each tenant gets isolated streams
// workflows.tenant_a::document_approval.places.draft.tokens
// workflows.tenant_b::document_approval.places.draft.tokens
```

### Bulk Token Operations

```graphql
mutation BulkTokenTransition {
  bulkTransitionTokens(input: {
    workflowId: "document_approval",
    fromPlace: "draft",
    toPlace: "review",
    transitionId: "bulk_submit",
    filter: {
      metadata: { department: "engineering" }
    }
  }) {
    success
    processedCount
    failedCount
    errors
  }
}
```

### Cross-Workflow Token Queries

```graphql
query CrossWorkflowTokens {
  findTokens(filter: {
    workflowIds: ["document_approval", "code_review"],
    places: ["review"],
    metadata: { priority: "high" }
  }) {
    id
    workflowId
    place
    data
    natsSequence
  }
}
```

### Workflow Analytics

```graphql
query WorkflowAnalytics {
  workflowAnalytics(workflowId: "document_approval") {
    totalTokens
    tokensPerPlace {
      place
      count
    }
    averageTransitionTime
    throughput {
      period: "last_24h"
      tokensCompleted: 45
      averageProcessingTime: "2h 15m"
    }
  }
}
```

## Configuration

### Environment Variables

```bash
# NATS Configuration
NATS_URL=nats://localhost:4222
NATS_CLUSTER_ID=circuit-breaker-cluster
NATS_CLIENT_ID=circuit-breaker-client

# Stream Configuration
NATS_MAX_MESSAGES=100000
NATS_MAX_BYTES=536870912  # 512MB
NATS_MAX_AGE=604800       # 7 days in seconds

# Connection Settings
NATS_CONNECTION_TIMEOUT=10
NATS_RECONNECT_BUFFER_SIZE=4194304  # 4MB
```

### Production Configuration

```rust
use circuit_breaker::NATSStorageConfig;
use std::time::Duration;

let production_config = NATSStorageConfig {
    nats_urls: vec![
        "nats://nats-1:4222".to_string(),
        "nats://nats-2:4222".to_string(),
        "nats://nats-3:4222".to_string(),
    ],
    default_max_messages: 1_000_000,
    default_max_bytes: 1024 * 1024 * 1024, // 1GB
    default_max_age: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
    consumer_timeout: Duration::from_secs(60),
    max_deliver: 5,
    connection_timeout: Duration::from_secs(30),
    reconnect_buffer_size: 8 * 1024 * 1024, // 8MB
};
```

## Performance Optimization

### Stream Tuning

```rust
// High-throughput configuration
let high_throughput_config = NATSStorageConfig {
    default_max_messages: 10_000_000,
    default_max_bytes: 5 * 1024 * 1024 * 1024, // 5GB
    consumer_timeout: Duration::from_secs(5),
    // Use memory storage for speed (with backup persistence)
    storage_type: StorageType::Memory,
    ..Default::default()
};
```

### Consumer Patterns

```rust
// Durable consumer for guaranteed processing
let durable_consumer = ConsumerConfig {
    durable_name: Some("workflow_processor".to_string()),
    deliver_policy: DeliverPolicy::All,
    ack_policy: AckPolicy::Explicit,
    max_deliver: 3,
    ..Default::default()
};

// Ephemeral consumer for real-time monitoring
let ephemeral_consumer = ConsumerConfig {
    durable_name: None,
    deliver_policy: DeliverPolicy::New,
    ack_policy: AckPolicy::None,
    ..Default::default()
};
```

## Monitoring and Observability

### Stream Health Monitoring

```rust
use circuit_breaker::NATSStorage;

async fn monitor_stream_health(nats_storage: &NATSStorage) -> Result<(), Box<dyn std::error::Error>> {
    let stream_info = nats_storage.get_stream_info("WORKFLOW_DOCUMENT_APPROVAL").await?;
    
    println!("Stream Health Report:");
    println!("  Messages: {}", stream_info.state.messages);
    println!("  Bytes: {}", stream_info.state.bytes);
    println!("  First Sequence: {}", stream_info.state.first_seq);
    println!("  Last Sequence: {}", stream_info.state.last_seq);
    println!("  Consumer Count: {}", stream_info.state.consumer_count);
    
    // Alert if message count is growing too fast
    if stream_info.state.messages > 1_000_000 {
        println!("⚠️  High message count detected!");
    }
    
    Ok(())
}
```

### GraphQL Monitoring Queries

```graphql
query SystemHealth {
  natsSystemHealth {
    connectedServers
    totalStreams
    totalConsumers
    totalMessages
    systemResources {
      memoryUsage
      diskUsage
      cpuUsage
    }
    clusterStatus {
      leader
      replicas
      inSync
    }
  }
}

query WorkflowHealth {
  workflowHealth(workflowId: "document_approval") {
    streamName
    messageCount
    consumerLag
    throughputPerMinute
    errorRate
    avgProcessingTime
  }
}
```

## Troubleshooting

### Common Issues

#### 1. Consumer Lag
```bash
# Check consumer lag
nats consumer info WORKFLOW_DOCUMENT_APPROVAL workflow_processor

# Solution: Scale consumers or optimize processing
```

#### 2. Stream Storage Growth
```bash
# Check stream storage
nats stream info WORKFLOW_DOCUMENT_APPROVAL

# Solution: Adjust retention policies or archive old data
```

#### 3. Connection Issues
```rust
// Robust connection handling
let nats_storage = NATSStorage::with_retry_config(
    config,
    RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        backoff_factor: 2.0,
    }
).await?;
```

### Debug Tools

```rust
// Enable detailed NATS logging
use tracing::Level;

tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .with_target(true)
    .init();

// Debug token transitions
let debug_storage = NATSStorage::with_debug_mode(config).await?;
```

### Performance Metrics

```graphql
query PerformanceMetrics {
  natsPerformanceMetrics(timeRange: "1h") {
    messagesPerSecond
    bytesPerSecond
    avgLatency
    p95Latency
    p99Latency
    errorRate
    connectionCount
    streamMetrics {
      streamName
      messageCount
      messageRate
      consumerCount
    }
  }
}
```

## Migration and Backup

### Backup Strategies

```bash
# Export workflow data
nats stream backup WORKFLOW_DOCUMENT_APPROVAL ./backup/

# Import workflow data
nats stream restore WORKFLOW_DOCUMENT_APPROVAL ./backup/
```

### Data Migration

```rust
use circuit_breaker::{NATSStorage, InMemoryStorage};

async fn migrate_to_nats(
    source: &InMemoryStorage,
    target: &NATSStorage
) -> Result<(), Box<dyn std::error::Error>> {
    // Export all workflows from in-memory storage
    let workflows = source.list_workflows().await?;
    
    for workflow in workflows {
        // Create workflow in NATS
        target.create_workflow(workflow.clone()).await?;
        
        // Migrate all tokens
        let tokens = source.list_tokens(&workflow.id).await?;
        for token in tokens {
            target.create_token_with_event(token, Some("migration".to_string())).await?;
        }
    }
    
    Ok(())
}
```

## Best Practices

### 1. Workflow Design
- Use descriptive workflow IDs that include version information
- Design workflows with clear state boundaries
- Implement proper error handling and retry logic

### 2. Subject Naming
- Follow consistent naming conventions
- Use hierarchical subjects for efficient filtering
- Avoid subject wildcards in production consumers

### 3. Resource Management
- Set appropriate stream limits based on expected load
- Monitor consumer lag and processing times
- Implement proper cleanup for completed workflows

### 4. Security
- Use NATS authentication and authorization
- Encrypt sensitive data in token payloads
- Implement audit logging for compliance

### 5. Scalability
- Design for horizontal scaling with multiple consumers
- Use partitioned streams for high-volume workflows
- Implement proper load balancing across NATS servers

## Example: Complete Workflow Implementation

```rust
use circuit_breaker::{NATSStorage, NATSStorageConfig, WorkflowDefinition, Token, PlaceId};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize NATS storage
    let config = NATSStorageConfig::default();
    let storage = std::sync::Arc::new(NATSStorage::new(config).await?);
    
    // Create a document approval workflow
    let workflow = WorkflowDefinition {
        id: "document_approval_v2".to_string(),
        name: "Document Approval Process".to_string(),
        places: vec![
            PlaceId::from("draft"),
            PlaceId::from("review"),
            PlaceId::from("approved"),
            PlaceId::from("published"),
        ],
        initial_place: PlaceId::from("draft"),
        // ... other workflow configuration
    };
    
    // Create the workflow
    storage.create_workflow(workflow).await?;
    
    // Create a token for a new document
    let mut token = Token::new("document_approval_v2", PlaceId::from("draft"));
    token.data = serde_json::json!({
        "title": "System Architecture Document",
        "author": "engineering-team",
        "priority": "high",
        "department": "engineering"
    });
    
    // Store token with NATS event tracking
    let stored_token = storage.create_token_with_event(
        token,
        Some("document_created".to_string())
    ).await?;
    
    println!("Created token: {} with NATS sequence: {}", 
        stored_token.id, 
        stored_token.nats_sequence.unwrap_or(0)
    );
    
    // Transition the token
    let transitioned_token = storage.transition_token_with_event(
        stored_token,
        PlaceId::from("review"),
        "submit_for_review".into(),
        Some("author".to_string())
    ).await?;
    
    println!("Token transitioned to: {}", transitioned_token.place.as_str());
    
    // Query tokens in review
    let review_tokens = storage.get_tokens_in_place(
        "document_approval_v2",
        "review"
    ).await?;
    
    println!("Tokens in review: {}", review_tokens.len());
    
    Ok(())
}
```

## NATS KV Storage for MCP Server

The Circuit Breaker MCP Server utilizes NATS KV (Key-Value) stores for high-performance session management, token caching, and security operations. This provides a unified storage solution without requiring additional Redis infrastructure.

### KV Store Types

Circuit Breaker uses multiple specialized KV stores:

```rust
use async_nats::jetstream::kv;
use std::time::Duration;

// Session token storage (1-hour TTL)
let session_kv_store = jetstream
    .create_key_value(kv::Config {
        bucket: "session_tokens".to_string(),
        max_age: Duration::from_hours(1),
        storage: StorageType::Memory,
        ..Default::default()
    })
    .await?;

// High-performance token cache (15-minute TTL)
let token_cache_kv_store = jetstream
    .create_key_value(kv::Config {
        bucket: "token_cache".to_string(),
        max_age: Duration::from_minutes(15),
        storage: StorageType::Memory,
        ..Default::default()
    })
    .await?;

// Rate limiting counters (1-hour TTL)
let rate_limit_kv_store = jetstream
    .create_key_value(kv::Config {
        bucket: "rate_limits".to_string(),
        max_age: Duration::from_hours(1),
        storage: StorageType::Memory,
        ..Default::default()
    })
    .await?;
```

### Session Token Management

#### Storing Session Tokens

```rust
use serde_json;
use sha2::{Sha256, Digest};

// Store session token with metadata
async fn store_session_token(
    kv_store: &Store,
    token: &str,
    session_data: &SessionTokenData,
) -> Result<(), Box<dyn std::error::Error>> {
    let token_hash = hash_token(token);
    let key = format!("session:{}", token_hash);
    let value = serde_json::to_vec(session_data)?;
    
    kv_store.put(&key, value.into()).await?;
    Ok(())
}

// Retrieve session token
async fn get_session_token(
    kv_store: &Store,
    token: &str,
) -> Result<Option<SessionTokenData>, Box<dyn std::error::Error>> {
    let token_hash = hash_token(token);
    let key = format!("session:{}", token_hash);
    
    match kv_store.get(&key).await {
        Ok(Some(entry)) => {
            let session_data: SessionTokenData = serde_json::from_slice(&entry.value)?;
            if session_data.expires_at > Utc::now() {
                Ok(Some(session_data))
            } else {
                // Auto-cleanup expired token
                let _ = kv_store.delete(&key).await;
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

#### Session Data Structure

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenData {
    pub token_id: String,
    pub installation_id: String,
    pub app_id: String,
    pub permissions: MCPPermissions,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
```

### Token Caching Patterns

#### Cache-First Validation

```rust
// Fast token validation using cache-first approach
async fn validate_token_cached(
    cache_kv: &Store,
    session_kv: &Store,
    token: &str,
) -> Result<SessionTokenData, Box<dyn std::error::Error>> {
    let token_hash = hash_token(token);
    
    // Try cache first (fast path)
    if let Some(session_data) = get_cached_session(&cache_kv, &token_hash).await? {
        return Ok(session_data);
    }
    
    // Fallback to main session store
    if let Some(session_data) = get_session_token(&session_kv, token).await? {
        // Update cache for future requests
        cache_session_token(&cache_kv, &token_hash, &session_data).await?;
        return Ok(session_data);
    }
    
    Err("Token not found or expired".into())
}
```

#### Cache Invalidation

```rust
// Invalidate cached token
async fn invalidate_cached_token(
    cache_kv: &Store,
    token_hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = format!("token:{}", token_hash);
    cache_kv.delete(&key).await?;
    Ok(())
}

// Add token to revocation list
async fn revoke_token(
    cache_kv: &Store,
    token_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = format!("revoked:{}", token_id);
    let value = b"revoked";
    
    // Store with TTL (automatically expires after 24 hours)
    cache_kv.put(&key, value.to_vec().into()).await?;
    Ok(())
}
```

### Rate Limiting with KV Store

#### Implementing Rate Limits

```rust
use chrono::Utc;

async fn check_rate_limit(
    rate_kv: &Store,
    installation_id: &str,
    resource: &str,
    limit: i64,
    window_seconds: i64,
) -> Result<RateLimitResult, Box<dyn std::error::Error>> {
    let window_start = Utc::now().timestamp() / window_seconds;
    let key = format!("rate:{}:{}:{}", installation_id, resource, window_start);
    
    // Get current count
    let current = match rate_kv.get(&key).await {
        Ok(Some(entry)) => {
            String::from_utf8_lossy(&entry.value).parse::<i64>().unwrap_or(0)
        }
        _ => 0,
    };
    
    if current >= limit {
        return Ok(RateLimitResult::Limited {
            limit,
            current,
            reset_at: DateTime::from_timestamp((window_start + 1) * window_seconds, 0).unwrap(),
        });
    }
    
    // Increment counter
    let new_count = current + 1;
    rate_kv.put(&key, new_count.to_string().into()).await?;
    
    Ok(RateLimitResult::Allowed {
        limit,
        remaining: limit - new_count,
        reset_at: DateTime::from_timestamp((window_start + 1) * window_seconds, 0).unwrap(),
    })
}

#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed {
        limit: i64,
        remaining: i64,
        reset_at: DateTime<Utc>,
    },
    Limited {
        limit: i64,
        current: i64,
        reset_at: DateTime<Utc>,
    },
}
```

### Project Context Caching

#### Caching Project Structure

```rust
// Cache project file structure
async fn cache_project_structure(
    cache_kv: &Store,
    project_id: i64,
    structure: &ProjectStructure,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = format!("project:{}:structure", project_id);
    let value = serde_json::to_vec(structure)?;
    
    cache_kv.put(&key, value.into()).await?;
    Ok(())
}

// Get cached project structure
async fn get_cached_project_structure(
    cache_kv: &Store,
    project_id: i64,
) -> Result<Option<ProjectStructure>, Box<dyn std::error::Error>> {
    let key = format!("project:{}:structure", project_id);
    
    match cache_kv.get(&key).await {
        Ok(Some(entry)) => {
            let structure: ProjectStructure = serde_json::from_slice(&entry.value)?;
            Ok(Some(structure))
        }
        _ => Ok(None),
    }
}
```

### KV Store Monitoring

#### Health Checks

```rust
// Monitor KV store health
async fn check_kv_store_health(
    kv_store: &Store,
) -> Result<KVStoreHealth, Box<dyn std::error::Error>> {
    let status = kv_store.status().await?;
    
    Ok(KVStoreHealth {
        bucket_name: status.bucket.clone(),
        entry_count: status.values,
        byte_size: status.bytes,
        is_healthy: status.values < 1_000_000, // Alert if too many entries
    })
}

#[derive(Debug)]
pub struct KVStoreHealth {
    pub bucket_name: String,
    pub entry_count: u64,
    pub byte_size: u64,
    pub is_healthy: bool,
}
```

#### Performance Metrics

```rust
// Track KV operations
async fn track_kv_operation(
    operation: &str,
    duration: Duration,
    success: bool,
) {
    println!("KV Operation: {} took {}ms, success: {}", 
        operation, 
        duration.as_millis(), 
        success
    );
}

// Example usage with timing
async fn timed_kv_get(
    kv_store: &Store,
    key: &str,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let result = kv_store.get(key).await;
    let duration = start.elapsed();
    
    track_kv_operation("get", duration, result.is_ok()).await;
    
    match result {
        Ok(Some(entry)) => Ok(Some(entry.value.to_vec())),
        Ok(None) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

### Configuration Best Practices

#### Production KV Store Configuration

```rust
use async_nats::jetstream::kv;

// High-performance session storage
let session_config = kv::Config {
    bucket: "session_tokens".to_string(),
    max_age: Duration::from_hours(1),
    storage: StorageType::Memory, // Fast access
    max_bytes: 100 * 1024 * 1024, // 100MB limit
    max_value_size: 1024 * 1024,  // 1MB per entry
    replicas: 3, // High availability
    ..Default::default()
};

// Cache with shorter TTL
let cache_config = kv::Config {
    bucket: "token_cache".to_string(),
    max_age: Duration::from_minutes(15),
    storage: StorageType::Memory,
    max_bytes: 50 * 1024 * 1024, // 50MB limit
    replicas: 2,
    ..Default::default()
};

// Rate limiting store
let rate_limit_config = kv::Config {
    bucket: "rate_limits".to_string(),
    max_age: Duration::from_hours(1),
    storage: StorageType::Memory,
    max_bytes: 10 * 1024 * 1024, // 10MB limit
    replicas: 1, // Can recreate if lost
    ..Default::default()
};
```

#### Environment Configuration

```bash
# KV Store Configuration
NATS_KV_SESSION_MAX_AGE=3600      # 1 hour
NATS_KV_CACHE_MAX_AGE=900         # 15 minutes
NATS_KV_RATE_LIMIT_MAX_AGE=3600   # 1 hour

# Memory limits
NATS_KV_SESSION_MAX_BYTES=104857600    # 100MB
NATS_KV_CACHE_MAX_BYTES=52428800       # 50MB
NATS_KV_RATE_LIMIT_MAX_BYTES=10485760  # 10MB

# Replication
NATS_KV_SESSION_REPLICAS=3
NATS_KV_CACHE_REPLICAS=2
NATS_KV_RATE_LIMIT_REPLICAS=1
```

### Integration Example

#### Complete MCP Server KV Integration

```rust
use async_nats::jetstream::{Context, kv};
use std::sync::Arc;

pub struct MCPServerKVManager {
    session_kv: Store,
    cache_kv: Store,
    rate_limit_kv: Store,
}

impl MCPServerKVManager {
    pub async fn new(jetstream: Context) -> Result<Self, Box<dyn std::error::Error>> {
        // Create all KV stores
        let session_kv = jetstream.create_key_value(kv::Config {
            bucket: "mcp_sessions".to_string(),
            max_age: Duration::from_hours(1),
            storage: StorageType::Memory,
            replicas: 3,
            ..Default::default()
        }).await?;
        
        let cache_kv = jetstream.create_key_value(kv::Config {
            bucket: "mcp_cache".to_string(),
            max_age: Duration::from_minutes(15),
            storage: StorageType::Memory,
            replicas: 2,
            ..Default::default()
        }).await?;
        
        let rate_limit_kv = jetstream.create_key_value(kv::Config {
            bucket: "mcp_rate_limits".to_string(),
            max_age: Duration::from_hours(1),
            storage: StorageType::Memory,
            replicas: 1,
            ..Default::default()
        }).await?;
        
        Ok(Self {
            session_kv,
            cache_kv,
            rate_limit_kv,
        })
    }
    
    // All session, cache, and rate limiting methods here...
}
```

This comprehensive NATS usage guide provides everything needed to effectively use NATS JetStream with Circuit Breaker for distributed workflow management and secure MCP server operations, from basic setup to advanced production configurations and troubleshooting.
