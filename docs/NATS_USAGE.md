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
