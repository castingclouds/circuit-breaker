# NATS Implementation for Circuit Breaker Workflows

## Implementation Status

✅ **COMPLETED**: The NATS integration has been successfully implemented and is ready for use.

### What's Been Built

- **NATS Storage Backend**: Complete `NATSStorage` implementation of the `WorkflowStorage` trait
- **Enhanced Token Model**: Extended `Token` struct with NATS-specific fields (`nats_sequence`, `nats_timestamp`, `nats_subject`, `transition_history`)
- **Stream Management**: Automatic NATS stream creation and configuration per workflow
- **GraphQL Integration**: NATS-specific queries and mutations (`natsToken`, `createWorkflowInstance`, `transitionTokenWithNats`, `tokensInPlace`)
- **Event Streaming**: Real-time token transition events with NATS JetStream
- **Error Handling**: Robust error conversion and handling for NATS operations
- **Demo Example**: Complete working example in `examples/rust/nats_demo.rs`

### Key Features

- **Backward Compatibility**: Existing workflows continue to work with in-memory storage
- **Streaming Architecture**: Tokens stored as messages in workflow-specific NATS streams
- **Enhanced Tracking**: Detailed transition history with NATS metadata
- **Real-time Events**: Token lifecycle events published to NATS for real-time updates
- **Efficient Queries**: Place-based token queries optimized for NATS subjects
- **GraphQL API**: Full GraphQL support for NATS-enhanced operations

## Overview

This document outlines the integration of NATS JetStream into the Circuit Breaker workflow system to provide distributed, scalable token storage and management. NATS JetStream serves as the persistence and streaming backbone for workflow tokens, enabling dynamic workflow creation, reliable token transitions, and real-time event processing.

## Architecture Design

### Core Concept: Tokens as Streaming Messages

In the NATS implementation, workflow tokens become persistent messages in JetStream streams. Each "place" in a workflow corresponds to a specific NATS subject and associated stream, allowing tokens to move through workflows via publish/consume operations.

```
Traditional Circuit Breaker:
Token in Place A → Transition → Token in Place B

NATS JetStream Integration:
Message in Stream A → Consumer/Publisher → Message in Stream B
```

### Subject Hierarchy

We'll use a hierarchical subject structure that supports dynamic workflow creation:

```
workflow.{workflowId}.places.{placeName}.tokens
```

**Examples:**
- `workflow.550e8400-e29b-41d4-a716-446655440000.places.pending-approval.tokens`
- `workflow.6ba7b810-9dad-11d1-80b4-00c04fd430c8.places.quality-check.tokens`
- `workflow.6ba7b814-9dad-11d1-80b4-00c04fd430c8.places.verification-complete.tokens`

**Workflow ID Generation:**
- Workflow IDs are automatically generated UUIDs (v4 format)
- Users provide workflow names/types, but the system creates unique IDs
- This prevents ID conflicts in distributed environments
- UUIDs ensure global uniqueness across multiple NATS clusters

**UUID Benefits:**
- **Collision-Free**: No coordination needed between distributed services
- **Sortable**: UUID v4 with timestamp ordering for workflow history
- **Secure**: Unpredictable IDs prevent enumeration attacks
- **Standard**: Compatible with database primary keys and APIs
- **Immutable**: Once created, workflow IDs never change

**Usage Pattern:**
```rust
// User creates workflow with name, system generates UUID
let workflow_request = CreateWorkflowRequest {
    name: "invoice-processing",
    places: vec!["submitted", "approved", "paid"],
    initial_place: "submitted",
    initial_data: json!({"invoice_id": "INV-2024-001"})
};

// System responds with generated UUID
let workflow = create_workflow(workflow_request).await?;
// workflow.id = "550e8400-e29b-41d4-a716-446655440000"
// workflow.name = "invoice-processing"
```

**Benefits:**
- **Dynamic Workflows**: New workflow instances create their required streams on-demand
- **Targeted Consumption**: Services can subscribe to specific places across all workflows or specific workflows across all places
- **Clear Organization**: Hierarchical structure makes workflow state obvious
- **Scalable Routing**: NATS handles complex subject-based routing efficiently

## Token Structure in NATS

### Token Message Format

The existing `Token` struct has been extended with NATS-specific fields for enhanced tracking:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    // Existing core fields
    pub id: Uuid,
    pub workflow_id: String,
    pub place: PlaceId,
    pub data: serde_json::Value,
    pub metadata: TokenMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub history: Vec<HistoryEvent>,
    
    // New NATS-specific fields (optional for backward compatibility)
    pub nats_sequence: Option<u64>,           // NATS stream sequence number
    pub nats_timestamp: Option<DateTime<Utc>>, // NATS timestamp
    pub nats_subject: Option<String>,         // Current NATS subject
    pub transition_history: Vec<TransitionRecord>, // NATS transition tracking
}

// NATS-specific transition record for detailed tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub from_place: PlaceId,
    pub to_place: PlaceId,
    pub transition_id: TransitionId,
    pub timestamp: DateTime<Utc>,
    pub triggered_by: Option<String>,
    pub nats_sequence: Option<u64>,
    pub metadata: Option<serde_json::Value>,
}

// Existing TokenMetadata remains unchanged
pub type TokenMetadata = HashMap<String, serde_json::Value>;
```

### Backward Compatibility

The NATS fields are optional (`Option<T>`) to maintain backward compatibility:
- Existing tokens work without NATS metadata
- NATS fields are automatically populated when using `NATSStorage`
- Serialization skips empty/None fields to keep JSON clean
- Progressive migration from in-memory to NATS storage is seamless

### Enhanced Token Methods

New NATS-specific methods have been added to the `Token` implementation:

```rust
impl Token {
    // Set NATS metadata
    pub fn set_nats_metadata(&mut self, sequence: u64, timestamp: DateTime<Utc>, subject: String)
    
    // Get NATS subject for current place
    pub fn nats_subject_for_place(&self) -> String
    
    // Enhanced transition with NATS tracking
    pub fn transition_to_with_nats(&mut self, new_place: PlaceId, transition_id: TransitionId, triggered_by: Option<String>, nats_sequence: Option<u64>)
    
    // Check if token has NATS metadata
    pub fn has_nats_metadata(&self) -> bool
}
```

## Stream Configuration Strategy

### Dynamic Stream Creation

Streams are created automatically per workflow with our `WorkflowStreamManager`:

```rust
pub struct WorkflowStreamManager {
    jetstream: Context,
    config: NATSStorageConfig,
}

impl WorkflowStreamManager {
    pub async fn ensure_workflow_streams(&self, workflow_id: &str) -> Result<()> {
        let stream_name = format!("WORKFLOW_{}", workflow_id.to_uppercase());
        let subjects = vec![
            format!("workflows.{}.definition", workflow_id),
            format!("workflows.{}.places.*.tokens", workflow_id),
            format!("workflows.{}.events.transitions", workflow_id),
            format!("workflows.{}.events.lifecycle", workflow_id),
        ];

        // Check if stream already exists
        if let Ok(_) = self.jetstream.get_stream(&stream_name).await {
            return Ok(());
        }

        // Create new stream configuration
        let stream_config = stream::Config {
            name: stream_name.clone(),
            subjects,
            max_messages: self.config.default_max_messages,
            max_bytes: self.config.default_max_bytes,
            max_age: self.config.default_max_age,
            storage: stream::StorageType::File,
            num_replicas: 1,
            retention: stream::RetentionPolicy::Interest,
            discard: stream::DiscardPolicy::Old,
            duplicate_window: Duration::from_secs(120),
            ..Default::default()
        };

        self.jetstream.create_stream(stream_config).await?;
        Ok(())
    }
}
```

### Stream Naming Convention

Our actual implementation uses a simplified, workflow-centric approach:

- **Stream Name**: `WORKFLOW_{WORKFLOW_ID}` (uppercase)
- **Subjects**:
  - `workflows.{workflow_id}.definition` - Workflow definitions
  - `workflows.{workflow_id}.places.*.tokens` - All tokens in any place
  - `workflows.{workflow_id}.events.transitions` - Transition events
  - `workflows.{workflow_id}.events.lifecycle` - Workflow lifecycle events

**Key Benefits:**
- **Single Stream per Workflow**: Simplifies management and reduces NATS overhead
- **Wildcard Support**: `places.*` allows efficient place-based filtering
- **Event Separation**: Dedicated subjects for different event types
- **Consistent Naming**: Uppercase stream names follow NATS conventions

**Stream Discovery Pattern:**
```rust
// Find all streams for a specific workflow
let workflow_pattern = format!("workflow_{}_places_*_tokens", workflow_id.hyphenated());

// Find all streams for a specific place across workflows  
let place_pattern = format!("workflow_*_places_{}_tokens", place_name);

// Find all workflow streams
let all_pattern = "workflow_*_places_*_tokens";
```

This convention provides:
- Unique stream identification
- Clear workflow and place association
- Consistent naming across the system
- Easy programmatic stream discovery
- Prevents naming conflicts in multi-tenant environments

## Token Operations

The NATS implementation provides enhanced token operations through the `NATSStorage` struct:

### 1. Creating Tokens (Injecting into Workflows)

```rust
// Create token with NATS event tracking
pub async fn create_token_with_event(&self, mut token: Token, triggered_by: Option<String>) -> Result<Token> {
    self.ensure_stream(&token.workflow_id).await?;

    let now = Utc::now();
    
    // Add creation event to transition history
    let creation_record = TransitionRecord {
        from_place: token.place.clone(),
        to_place: token.place.clone(),
        transition_id: TransitionId::from("create"),
        timestamp: now,
        triggered_by: triggered_by.clone(),
        nats_sequence: Some(0),
        metadata: Some(serde_json::json!({
            "event_type": "token_created",
            "workflow_id": token.workflow_id
        })),
    };

    token.add_transition_record(creation_record);
    
    // Publish creation event
    let event_subject = format!("workflows.{}.events.lifecycle", token.workflow_id);
    let event_payload = serde_json::json!({
        "event_type": "token_created",
        "token_id": token.id,
        "workflow_id": token.workflow_id,
        "place": token.place.as_str(),
        "timestamp": now,
        "triggered_by": triggered_by
    });

    self.jetstream.publish(event_subject, serde_json::to_vec(&event_payload)?.into()).await?;
    
    // Publish the token itself
    self.publish_token(&token).await?;
    
    Ok(token)
}
```

### 2. Token Transitions

Transition tokens with NATS event publishing and tracking:

```rust
pub async fn transition_token_with_event(
    &self,
    mut token: Token,
    new_place: PlaceId,
    transition_id: TransitionId,
    triggered_by: Option<String>,
) -> Result<Token> {
    let old_place = token.place.clone();
    
    // Perform the transition with NATS tracking
    token.transition_to_with_nats(
        new_place.clone(),
        transition_id.clone(),
        triggered_by.clone(),
        None, // Sequence will be set by NATS
    );

    // Publish transition event
    let event_subject = format!("workflows.{}.events.transitions", token.workflow_id);
    let event_payload = serde_json::json!({
        "event_type": "token_transitioned",
        "token_id": token.id,
        "workflow_id": token.workflow_id,
        "from_place": old_place.as_str(),
        "to_place": new_place.as_str(),
        "transition_id": transition_id.as_str(),
        "timestamp": Utc::now(),
        "triggered_by": triggered_by
    });

    self.jetstream.publish(event_subject, serde_json::to_vec(&event_payload)?.into()).await?;

    // Republish the token to its new place
    self.publish_token(&token).await?;

    Ok(token)
}
```

### 3. Querying Tokens

NATS storage provides efficient token queries:

```rust
// Get tokens currently in a specific place
pub async fn get_tokens_in_place(&self, workflow_id: &str, place_id: &str) -> Result<Vec<Token>> {
    let stream_name = self.stream_manager().stream_name(workflow_id);
    let stream = self.jetstream.get_stream(&stream_name).await?;

    let consumer_config = consumer::pull::Config {
        durable_name: Some(format!("place_tokens_consumer_{}_{}", workflow_id, place_id)),
        filter_subject: format!("workflows.{}.places.{}.tokens", workflow_id, place_id),
        ..Default::default()
    };

    let consumer = stream.create_consumer(consumer_config).await?;
    let mut tokens = Vec::new();
    let mut batch = consumer.batch().max_messages(1000).messages().await?;
    
    while let Some(message) = batch.next().await {
        let message = message?;
        if let Ok(token) = serde_json::from_slice::<Token>(&message.payload) {
            tokens.push(token);
        }
        message.ack().await?;
    }

    Ok(tokens)
}

// Find token by ID with known workflow (more efficient)
pub async fn find_token(&self, workflow_id: &str, token_id: &Uuid) -> Result<Option<Token>> {
    self.get_token_from_workflow(token_id, workflow_id).await
}
    
```

## GraphQL Integration

The NATS implementation extends the existing GraphQL API with enhanced types and operations:

### Enhanced GraphQL Types

```graphql
# Enhanced token type with NATS metadata
type NATSToken {
  id: ID!
  workflowId: String!
  place: String!
  data: JSON!
  metadata: JSON!
  createdAt: String!
  updatedAt: String!
  history: [HistoryEvent!]!
  
  # NATS-specific fields
  natsSequence: String
  natsTimestamp: String
  natsSubject: String
  transitionHistory: [TransitionRecord!]!
}

# NATS transition record with detailed tracking
type TransitionRecord {
  fromPlace: String!
  toPlace: String!
  transitionId: String!
  timestamp: String!
  triggeredBy: String
  natsSequence: String
  metadata: JSON
}

# Input types for NATS operations
input CreateWorkflowInstanceInput {
  workflowId: String!
  initialData: JSON
  metadata: JSON
  triggeredBy: String
}

input TransitionTokenWithNATSInput {
  tokenId: String!
  transitionId: String!
  newPlace: String!
  triggeredBy: String
  data: JSON
}
```

### GraphQL Queries and Mutations

The NATS implementation adds several enhanced GraphQL operations:

```graphql
type Query {
  # Enhanced token queries with NATS metadata
  natsToken(id: String!): NATSToken
  tokensInPlace(workflowId: String!, placeId: String!): [NATSToken!]!
  findToken(workflowId: String!, tokenId: String!): NATSToken
  
  # Existing queries continue to work
  token(id: String!): Token
  tokens(workflowId: String): [Token!]!
  workflows: [Workflow!]!
}

type Mutation {
  # NATS-enhanced token operations
  createWorkflowInstance(input: CreateWorkflowInstanceInput!): NATSToken!
  transitionTokenWithNats(input: TransitionTokenWithNATSInput!): NATSToken!
  
  # Existing mutations continue to work
  createWorkflow(input: WorkflowDefinitionInput!): Workflow!
  createToken(input: TokenCreateInput!): Token!
  fireTransition(input: TransitionFireInput!): Token!
}
```

### Example Usage

```graphql
# Create a workflow instance with NATS tracking
mutation CreateInstance {
  createWorkflowInstance(input: {
    workflowId: "document_review"
    initialData: {
      title: "Quarterly Report"
      priority: "high"
    }
    metadata: {
      department: "finance"
      created_by: "user@example.com"
    }
    triggeredBy: "api_client"
  }) {
    id
    workflowId
    place
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

# Transition with NATS event tracking
mutation TransitionToken {
  transitionTokenWithNats(input: {
    tokenId: "token-uuid-here"
    transitionId: "approve"
    newPlace: "approved"
    triggeredBy: "manager@example.com"
  }) {
    id
    place
    natsSequence
    transitionHistory {
      transitionId
      timestamp
      triggeredBy
    }
  }
}

# Query tokens in a specific place (NATS-optimized)
query TokensInPlace {
  tokensInPlace(workflowId: "document_review", placeId: "pending_approval") {
    id
    data
    natsSequence
    transitionHistory {
      fromPlace
      toPlace
      timestamp
    }
  }
}
  
  # List workflow instances with optional filtering
  workflowInstances(status: WorkflowStatus, nameContains: String, limit: Int = 10): [WorkflowInstance!]!
  
  # Get tokens in a specific place
  tokensInPlace(workflowId: ID!, placeName: String!, limit: Int = 10): [Token!]!
  
  # Get workflow stream information
  workflowStreams(workflowId: ID!): [StreamInfo!]!
  
  # Get transition history for a token
  tokenTransitionHistory(tokenId: ID!): [TransitionRecord!]!
}
</edits>

<edits>

<old_text>
# NATS-aware mutations  
type Mutation {
  # Create new workflow instance with initial token (UUID auto-generated)
  createWorkflowInstance(input: CreateWorkflowInstanceInput!): WorkflowInstance!
  
  # Inject token into existing workflow by UUID
  injectToken(input: InjectTokenInput!): Token!
  
  # Request token transition using workflow UUID
  requestTokenTransition(input: TokenTransitionInput!): TransitionResult!
}

input CreateWorkflowInstanceInput {
  workflowName: String! # Human-readable workflow name/type
  description: String # Optional description
  workflowDefinition: String # Reference to workflow schema
  places: [String!]! # Places to create streams for
  initialPlace: String!
  initialTokenData: JSON!
  # Note: workflowId will be auto-generated as UUID
}

input InjectTokenInput {
  workflowId: ID! # UUID format
  placeName: String!
  tokenData: JSON!
  metadata: JSON # Optional additional metadata
}

input TokenTransitionInput {
  tokenId: ID!
  fromWorkflow: ID! # UUID format
  fromPlace: String!
  toPlace: String!
  transitionId: String!
  metadata: JSON # Optional additional context
}

type StreamInfo {
  name: String!
  subject: String!
  messageCount: Int!
  byteSize: Int!
  lastActivity: DateTime
}

# NATS-aware mutations
type Mutation {
  # Create new workflow instance with initial token
  createWorkflowInstance(input: CreateWorkflowInstanceInput!): WorkflowInstance!
  
  # Inject token into existing workflow
  injectToken(input: InjectTokenInput!): Token!
  
  # Request token transition
  requestTokenTransition(input: TokenTransitionInput!): TransitionResult!
}

input CreateWorkflowInstanceInput {
  workflowName: String! # Human-readable workflow name/type
  workflowDefinition: String # Reference to workflow schema
  places: [String!]! # Places to create streams for
  initialPlace: String!
  initialTokenData: JSON!
  # Note: workflowId will be auto-generated as UUID
}

input TokenTransitionInput {
  tokenId: ID!
  fromWorkflow: ID! # UUID format
  fromPlace: String!
  toPlace: String!
  transitionId: String!
  metadata: JSON # Optional additional context
}

type TransitionResult {
  success: Boolean!
  message: String
  token: Token
}

# Real-time subscriptions
type Subscription {
  # Subscribe to token events in a place
  tokenEvents(workflowId: ID!, placeName: String!): TokenEvent!
  
  # Subscribe to all events for a specific token
  tokenUpdates(tokenId: ID!): TokenEvent!
  
  # Subscribe to workflow-wide events
  workflowEvents(workflowId: ID!): WorkflowEvent!
}

type TokenEvent {
  eventType: TokenEventType!
  token: Token!
  timestamp: DateTime!
  source: String # Service that triggered the event
}

enum TokenEventType {
  CREATED
  TRANSITIONED
  UPDATED
  AGENT_PROCESSED
  FUNCTION_EXECUTED
}

type WorkflowEvent {
  eventType: WorkflowEventType!
  workflowId: ID!  # UUID format
  data: JSON!
  timestamp: DateTime!
}

enum WorkflowEventType {
  INSTANCE_CREATED
  STREAM_CREATED
  TOKEN_INJECTED
  WORKFLOW_COMPLETED
}
```

### GraphQL Resolver Implementation

```rust
// Resolver for tokens in a place
async fn tokens_in_place(
    ctx: &Context<'_>,
    workflow_id: ID,  // UUID format
    place_name: String,
    limit: Option<i32>,
) -> Result<Vec<Token>> {
    let nats_client = ctx.data::<NATSWorkflowClient>()?;
    let workflow_uuid = Uuid::parse_str(&workflow_id)?;
    let tokens = nats_client.get_tokens_in_place(&workflow_uuid, &place_name).await?;
    
    let limit = limit.unwrap_or(10) as usize;
    Ok(tokens.into_iter().take(limit).map(Token::from).collect())
}

// Resolver for creating workflow instances
async fn create_workflow_instance(
    ctx: &Context<'_>,
    input: CreateWorkflowInstanceInput,
) -> Result<WorkflowInstance> {
    let nats_client = ctx.data::<NATSWorkflowClient>()?;
    
    // Generate unique workflow ID
    let workflow_id = Uuid::new_v4();
    
    // Create streams for all places
    nats_client.ensure_workflow_streams(&workflow_id, &input.places).await?;
    
    // Create initial token
    let token_id = nats_client.create_token(
        &workflow_id,
        &input.initial_place,
        input.initial_token_data,
    ).await?;
    
    Ok(WorkflowInstance {
        id: workflow_id.to_string(),
        name: input.workflow_name,
        description: input.description,
        places: input.places,
        created_at: Utc::now(),
        status: WorkflowStatus::Active,
        initial_token_id: Some(token_id),
    })
}

// Subscription for token events
async fn token_events(
    ctx: &Context<'_>,
    workflow_id: ID,  // UUID format
    place_name: String,
) -> Result<impl Stream<Item = TokenEvent>> {
    let nats_client = ctx.data::<NATSWorkflowClient>()?.clone();
    let workflow_uuid = Uuid::parse_str(&workflow_id)?;
    let subject = format!("workflow.{}.places.{}.tokens", workflow_uuid, place_name);
    
    // Create NATS subscription for real-time events
    let subscription = nats_client.subscribe_to_place(&workflow_uuid, &place_name).await?;
    
    Ok(subscription.map(|message| {
        // Convert NATS message to GraphQL event
        let token: NATSToken = serde_json::from_slice(&message.payload)?;
        TokenEvent {
            event_type: TokenEventType::Created, // Determine based on context
            token: Token::from(token),
            timestamp: Utc::now(),
            source: "nats-stream".to_string(),
        }
    }))
}
```

## Circuit Breaker Component Integration

### 1. Agent Integration

Agents can process tokens from NATS streams and update them with results:

```rust
pub async fn execute_agent_on_token(
    &self,
    agent_id: &str,
    token: &mut NATSToken,
) -> Result<AgentExecutionResult> {
    // Execute agent with token data
    let agent_result = self.agent_engine.execute_agent(
        agent_id,
        &token.data,
        &token.metadata.custom_fields,
    ).await?;
    
    // Store agent output in token metadata
    token.metadata.agent_outputs.insert(
        agent_id.to_string(),
        serde_json::to_value(&agent_result.output)?,
    );
    
    // Update token timestamp
    token.metadata.updated_at = Utc::now();
    
    Ok(agent_result)
}
```

### 2. Function Runner Integration

Functions can be triggered by token events and publish results back to NATS:

```rust
// Function triggered by token creation in specific place
pub async fn on_token_created(
    &self,
    event: TokenEvent,
) -> Result<()> {
    if let Some(function_config) = self.get_function_for_place(&event.token.current_place) {
        // Execute function with token data
        let function_result = self.function_engine.execute_function(
            &function_config.function_id,
            &event.token.data,
        ).await?;
        
        // Update token with function output
        let mut token = event.token;
        token.metadata.function_outputs.insert(
            function_config.function_id.clone(),
            function_result.output,
        );
        
        // Republish updated token to same place
        self.nats_client.update_token_in_place(token).await?;
        
        // Optionally trigger transition based on function result
        if function_result.should_transition {
            let workflow_uuid = Uuid::parse_str(&token.workflow_id)?;
            self.nats_client.transition_token(
                &token.id,
                &workflow_uuid,  // UUID, not string
                &token.current_place,
                &function_config.target_place,
                "function_triggered",
            ).await?;
        }
    }
    
    Ok(())
}
```

### 3. Rules Engine Integration

Rules can evaluate tokens including NATS-specific metadata:

```rust
impl Rule {
    // Rule that checks transition history
    pub fn has_transitioned_from(place_name: &str) -> Self {
        Rule::custom(
            format!("has_transitioned_from_{}", place_name),
            format!("Token has transitioned from place {}", place_name),
            move |token: &NATSToken| {
                token.transition_history.iter()
                    .any(|record| record.from_place == place_name)
            }
        )
    }
    
    // Rule that checks agent outputs
    pub fn agent_output_exists(agent_id: &str) -> Self {
        Rule::custom(
            format!("agent_output_{}", agent_id),
            format!("Agent {} has processed this token", agent_id),
            move |token: &NATSToken| {
                token.metadata.agent_outputs.contains_key(agent_id)
            }
        )
    }
    
    // Rule that checks function execution results
    pub fn function_succeeded(function_id: &str) -> Self {
        Rule::custom(
            format!("function_succeeded_{}", function_id),
            format!("Function {} completed successfully", function_id),
            move |token: &NATSToken| {
                token.metadata.function_outputs.get(function_id)
                    .and_then(|output| output.get("success"))
                    .and_then(|success| success.as_bool())
                    .unwrap_or(false)
            }
        )
    }
}
```

## Implementation Phases

### Phase 1: Core NATS Integration ✅ **COMPLETED**
- [x] NATS JetStream client setup and connection management
- [x] `NATSStorage` implementation of `WorkflowStorage` trait
- [x] Automatic stream creation and configuration per workflow
- [x] Enhanced `Token` struct with NATS-specific fields
- [x] Token transition mechanics with NATS event publishing
- [x] Stream naming conventions and subject hierarchy
- [x] Error handling with `anyhow` integration

### Phase 2: GraphQL API Extension ✅ **COMPLETED**
- [x] Extended GraphQL schema with `NATSToken` and `TransitionRecord` types
- [x] NATS-specific input types (`CreateWorkflowInstanceInput`, `TransitionTokenWithNATSInput`)
- [x] Enhanced resolver implementations (`natsToken`, `tokensInPlace`, `findToken`)
- [x] NATS-enhanced mutations (`createWorkflowInstance`, `transitionTokenWithNats`)
- [x] Schema creation functions for NATS integration
- [x] Backward compatibility with existing GraphQL operations

### Phase 3: Component Integration 🔄 **PARTIALLY COMPLETED**
- [x] NATS storage backend integration
- [x] Event-driven token lifecycle management
- [x] Real-time transition event publishing
- [ ] Agent execution with NATS tokens
- [ ] Function runner event triggers
- [ ] Rules engine NATS-aware rules
- [ ] Cross-component event coordination

### Phase 4: Advanced Features 📋 **PLANNED**
- [ ] Token search and indexing optimization
- [ ] Workflow analytics and monitoring dashboards
- [ ] Performance optimization and caching
- [ ] Advanced error handling and retry mechanisms
- [ ] Multi-tenant workflow isolation

### Phase 5: Production Features 📋 **PLANNED**
- [ ] NATS cluster configuration and high availability
- [ ] Security and authentication integration
- [ ] Comprehensive monitoring and observability
- [ ] Backup and disaster recovery procedures
- [ ] Load testing and performance benchmarking

## Operational Considerations

### Performance
- **Stream Retention**: Configure appropriate retention policies for each workflow type
- **Consumer Optimization**: Use appropriate consumer types (push vs pull) for different use cases
- **Message Batching**: Batch operations where possible to reduce NATS overhead
- **Connection Pooling**: Reuse NATS connections across services

### Security
- **Subject Permissions**: Use NATS authorization to control access to workflow streams
- **Token Encryption**: Encrypt sensitive token data at rest and in transit
- **Audit Logging**: Track all token operations for compliance and debugging

### Monitoring
- **Stream Metrics**: Monitor message counts, processing rates, and consumer lag
- **Token Lifecycle**: Track token creation, transitions, and completion rates
- **Error Rates**: Monitor failed transitions and retry patterns

### Scalability
- **Horizontal Scaling**: Multiple service instances can consume from the same streams
- **Workflow Partitioning**: Large workflows can be partitioned across multiple NATS clusters
- **Auto-scaling**: Scale consumers based on stream message backlog

## Getting Started with NATS Integration

### Prerequisites

1. **NATS Server with JetStream**: Ensure NATS server is running with JetStream enabled:
   ```bash
   nats-server --jetstream
   ```

2. **Dependencies**: The NATS integration is included in the main Circuit Breaker crate with the `async-nats` dependency.

### Basic Usage

#### 1. Setting up NATS Storage

```rust
use circuit_breaker::{NATSStorage, NATSStorageConfig, create_schema_with_nats};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure NATS storage
    let nats_config = NATSStorageConfig {
        nats_urls: vec!["nats://localhost:4222".to_string()],
        default_max_messages: 100_000,
        default_max_bytes: 512 * 1024 * 1024, // 512MB
        default_max_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
        consumer_timeout: Duration::from_secs(30),
        max_deliver: 3,
        connection_timeout: Duration::from_secs(10),
        reconnect_buffer_size: 4 * 1024 * 1024, // 4MB
    };

    // Create NATS storage instance
    let nats_storage = std::sync::Arc::new(
        NATSStorage::new(nats_config).await?
    );

    // Create GraphQL schema with NATS storage
    let schema = create_schema_with_nats(nats_storage);

    // Start your GraphQL server...
    Ok(())
}
```

#### 2. Using NATS-Enhanced GraphQL API

```graphql
# Create a workflow instance with NATS tracking
mutation {
  createWorkflowInstance(input: {
    workflowId: "document_review"
    initialData: {
      title: "Project Proposal"
      department: "engineering"
    }
    triggeredBy: "api_client"
  }) {
    id
    place
    natsSequence
    natsSubject
  }
}

# Query tokens in a specific place (NATS-optimized)
query {
  tokensInPlace(workflowId: "document_review", placeId: "draft") {
    id
    data
    natsSequence
    transitionHistory {
      fromPlace
      toPlace
      timestamp
      triggeredBy
    }
  }
}

# Perform NATS-tracked transitions
mutation {
  transitionTokenWithNats(input: {
    tokenId: "your-token-id"
    transitionId: "submit_for_review"
    newPlace: "review"
    triggeredBy: "user@example.com"
  }) {
    id
    place
    natsSequence
  }
}
```

### Running the Demo

A complete working example is available in the repository:

```bash
# Ensure NATS server is running
nats-server --jetstream

# Run the NATS demo
cargo run --example nats_demo
```

The demo will:
1. Start a Circuit Breaker server with NATS storage
2. Create a sample workflow
3. Create workflow instances with NATS tracking
4. Perform transitions with event publishing
5. Query tokens using NATS-optimized operations

### Migration from In-Memory Storage

The NATS integration is designed for seamless migration:

1. **Backward Compatibility**: Existing workflows continue to work
2. **Progressive Migration**: You can migrate workflows one at a time
3. **Dual Storage**: Run both in-memory and NATS storage simultaneously during transition
4. **GraphQL Compatibility**: All existing GraphQL operations continue to work

### Next Steps

- Review the demo code in `examples/rust/nats_demo.rs` for a complete working example
- Configure NATS cluster settings for production deployment
- Implement monitoring and alerting for NATS streams
- Consider integrating with existing agent and function systems

## Workflow Lifecycle Management

### UUID-Based Workflow Operations

The UUID-based workflow system provides clean lifecycle management:

```rust
// 1. Create workflow instance (UUID auto-generated)
pub async fn create_workflow_instance(
    &self,
    name: &str,
    places: Vec<String>,
    initial_place: &str,
    initial_data: serde_json::Value,
) -> Result<(Uuid, TokenId)> {
    let workflow_id = Uuid::new_v4();
    
    // Create NATS streams for all places
    self.ensure_workflow_streams(&workflow_id, &places).await?;
    
    // Create initial token
    let token_id = self.create_token(&workflow_id, initial_place, initial_data).await?;
    
    // Store workflow metadata in NATS KV store for discovery
    let workflow_meta = WorkflowMetadata {
        id: workflow_id,
        name: name.to_string(),
        places,
        created_at: Utc::now(),
        status: WorkflowStatus::Active,
    };
    
    self.store_workflow_metadata(&workflow_id, &workflow_meta).await?;
    
    Ok((workflow_id, token_id))
}

// 2. Query workflows by name pattern (multiple instances)
pub async fn find_workflows_by_name(&self, name_pattern: &str) -> Result<Vec<WorkflowMetadata>> {
    // Query NATS KV store for workflow metadata
    let workflows = self.kv.get_by_prefix("workflow_meta_").await?;
    
    Ok(workflows.into_iter()
        .filter(|w| w.name.contains(name_pattern))
        .collect())
}

// 3. Archive completed workflows  
pub async fn archive_workflow(&self, workflow_id: &Uuid) -> Result<()> {
    // Update workflow status
    let mut meta = self.get_workflow_metadata(workflow_id).await?;
    meta.status = WorkflowStatus::Archived;
    self.store_workflow_metadata(workflow_id, &meta).await?;
    
    // Optionally clean up streams after archival period
    // self.cleanup_workflow_streams(workflow_id).await?;
    
    Ok(())
}
```

### Multi-Tenant UUID Patterns

UUIDs enable clean multi-tenant separation:

```rust
// Tenant-scoped workflow creation
pub async fn create_tenant_workflow(
    &self,
    tenant_id: &Uuid,
    workflow_name: &str,
    places: Vec<String>,
) -> Result<Uuid> {
    let workflow_id = Uuid::new_v4();
    
    // Store tenant association
    let tenant_key = format!("tenant_{}:workflow_{}", tenant_id, workflow_id);
    self.kv.put(&tenant_key, workflow_id.as_bytes()).await?;
    
    // Create workflow with tenant context
    self.create_workflow_instance(workflow_name, places, "initial", json!({})).await?;
    
    Ok(workflow_id)
}

// Query workflows by tenant
pub async fn get_tenant_workflows(&self, tenant_id: &Uuid) -> Result<Vec<Uuid>> {
    let prefix = format!("tenant_{}:workflow_", tenant_id);
    let entries = self.kv.get_by_prefix(&prefix).await?;
    
    Ok(entries.into_iter()
        .map(|entry| Uuid::from_slice(&entry.value).unwrap())
        .collect())
}
```

### UUID Security Benefits

- **No Enumeration**: Attackers cannot guess workflow IDs
- **Tenant Isolation**: UUID-based access control prevents cross-tenant access
- **Audit Trails**: Immutable workflow IDs in all log entries
- **API Security**: GraphQL resolvers can validate UUID format before processing

This NATS implementation provides a robust, scalable foundation for distributed workflow processing while maintaining the flexibility and power of the Circuit Breaker architecture. The event-driven nature of NATS JetStream aligns perfectly with the agent and function execution patterns, creating a cohesive system for complex workflow automation.