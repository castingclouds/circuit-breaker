# Agent Configuration and Streaming Architecture

## Overview

The Circuit Breaker agent system provides a flexible framework for integrating AI agents into workflow execution. Agents can use different LLM providers (OpenAI, Anthropic, Google, Ollama) with configurable prompts, parameters, and real-time streaming responses.

## Architecture Design

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Agent Models              │    │  LLM Providers              │    │ Stream Manager             │
│                            │    │                             │    │                            │
│ • AgentDef                 │    │ • OpenAI                    │    │ • Multi-protocol           │
│ • LLMConfig                │    │ • Anthropic                 │    │ • Real-time                │
│ • Prompts                  │    │ • Google                    │    │ • Event-driven             │
│ • Conversations            │    │ • Ollama                    │    │ • GraphQL/WS               │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                                      │                                       │
         └───────────────────────┼───────────────────────┘
                                                 │
                                     ┌─────────────────┐
                                     │  Agent Engine              │
                                     │                            │
                                     │ • Execution                │
                                     │ • Integration              │
                                     │ • Workflow                 │
                                     │ • Storage                  │
                                     └─────────────────┘
```

### Data Flow

1. **Agent Definition** → Stored with LLM provider configuration
2. **Workflow Transition** → Triggers agent execution
3. **LLM Provider** → Generates streaming response
4. **Stream Manager** → Broadcasts events via multiple protocols
5. **Clients** → Receive real-time updates via GraphQL/WebSocket

## Agent Configuration

### Agent Definition Structure

```rust
pub struct AgentDefinition {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub llm_provider: LLMProvider,
    pub llm_config: LLMConfig,
    pub prompts: AgentPrompts,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### LLM Provider Configuration

#### OpenAI Configuration
```rust
LLMProvider::OpenAI {
    api_key: String,           // Required: OpenAI API key
    model: String,             // gpt-4, gpt-3.5-turbo, gpt-4-turbo
    base_url: Option<String>,  // Optional: For Azure OpenAI
}
```

**Supported Models:**
- `gpt-4` - Most capable, higher cost
- `gpt-4-turbo` - Faster GPT-4 variant
- `gpt-3.5-turbo` - Fast and cost-effective
- `gpt-3.5-turbo-16k` - Extended context window

**Environment Variables:**
```bash
OPENAI_API_KEY=sk-...
OPENAI_BASE_URL=https://api.openai.com/v1  # Optional
```

#### Anthropic Configuration
```rust
LLMProvider::Anthropic {
    api_key: String,    // Required: Anthropic API key
    model: String,      // claude-3-opus, claude-3-sonnet, claude-3-haiku
}
```

**Supported Models:**
- `claude-3-opus` - Most capable, highest cost
- `claude-3-sonnet` - Balanced performance and cost
- `claude-3-haiku` - Fastest, most cost-effective

**Environment Variables:**
```bash
ANTHROPIC_API_KEY=sk-ant-...
```

#### Google Gemini Configuration
```rust
LLMProvider::Google {
    api_key: String,    // Required: Google API key
    model: String,      // gemini-pro, gemini-pro-vision
}
```

**Supported Models:**
- `gemini-pro` - Text and code generation
- `gemini-pro-vision` - Multimodal (text + images)

**Environment Variables:**
```bash
GOOGLE_API_KEY=...
```

#### Ollama Configuration (Local Models)
```rust
LLMProvider::Ollama {
    base_url: String,   // Usually http://localhost:11434
    model: String,      // llama2, mistral, codellama, etc.
}
```

**Popular Local Models:**
- `llama2` - General purpose, good performance
- `llama2:13b` - Larger variant, better quality
- `mistral` - Fast and efficient
- `codellama` - Optimized for code generation
- `vicuna` - Fine-tuned for conversations
- `orca-mini` - Smaller, faster model

**Setup Requirements:**
```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Download models
ollama pull llama2
ollama pull mistral
ollama pull codellama

# Start Ollama service
ollama serve
```

#### Custom API Configuration
```rust
LLMProvider::Custom {
    api_key: Option<String>,
    base_url: String,
    model: String,
    headers: HashMap<String, String>,
}
```

### LLM Generation Parameters

```rust
pub struct LLMConfig {
    pub temperature: f32,              // 0.0-2.0, controls randomness
    pub max_tokens: Option<u32>,       // Maximum response length
    pub top_p: Option<f32>,            // 0.0-1.0, nucleus sampling
    pub frequency_penalty: Option<f32>, // -2.0-2.0, reduces repetition
    pub presence_penalty: Option<f32>,  // -2.0-2.0, encourages new topics
    pub stop_sequences: Vec<String>,    // Strings that stop generation
}
```

**Parameter Guidelines:**
- **Temperature**: `0.0` (deterministic) to `2.0` (very creative)
  - `0.0-0.3`: Factual, consistent responses
  - `0.4-0.7`: Balanced creativity and consistency
  - `0.8-2.0`: Creative, varied responses
- **Max Tokens**: Model-dependent limits
  - GPT-4: 8,192 or 32,768 (turbo)
  - Claude-3: 200,000
  - Gemini Pro: 30,720
- **Top P**: `1.0` (full vocabulary) to `0.1` (restricted)
- **Penalties**: Help reduce repetitive text

### Prompt Configuration

```rust
pub struct AgentPrompts {
    pub system: String,                    // Defines agent behavior and role
    pub user_template: String,             // Template for user messages
    pub context_instructions: Option<String>, // How to handle context
}
```

**Template Variables:**
- `{input_data}` - Input data from workflow
- `{token_id}` - Current workflow token ID
- `{current_place}` - Current workflow state
- `{token_metadata}` - Token metadata as JSON
- `{workflow_id}` - Workflow identifier
- `{history_count}` - Number of transitions

**Example System Prompt:**
```
You are a content review specialist working within a document approval workflow.
Your role is to analyze documents and provide detailed feedback on quality,
accuracy, and compliance with company standards.

Key responsibilities:
- Review document content for factual accuracy
- Check formatting and style consistency
- Identify potential compliance issues
- Provide actionable improvement suggestions

Always respond in a structured format with:
1. Overall assessment (APPROVE/NEEDS_REVISION/REJECT)
2. Specific issues found
3. Recommended actions
4. Confidence score (1-10)
```

**Example User Template:**
```
Please review the following document:

Document Type: {document_type}
Content: {input_data}
Current Status: {current_place}
Priority: {priority}

Token ID: {token_id}
Metadata: {token_metadata}

Provide your analysis following the structured format.
```

## Workflow Integration

### Agent-Enabled Transitions

Extend transition definitions to include agent execution:

```rust
pub struct TransitionDefinition {
    pub id: TransitionId,
    pub from_places: Vec<PlaceId>,
    pub to_place: PlaceId,
    pub conditions: Vec<String>,
    pub rules: Vec<Rule>,
    pub agent_execution: Option<AgentTransitionConfig>,  // NEW
}

pub struct AgentTransitionConfig {
    pub agent_id: AgentId,
    pub input_mapping: HashMap<String, String>,    // Map token data to agent input
    pub output_mapping: HashMap<String, String>,   // Map agent output to token
    pub required: bool,                            // Whether agent execution is required
    pub timeout_seconds: Option<u64>,             // Execution timeout
    pub retry_config: Option<AgentRetryConfig>,   // Retry on failures
}

pub struct AgentRetryConfig {
    pub max_attempts: u32,
    pub backoff_seconds: u64,
    pub retry_on_errors: Vec<String>,
}
```

### Execution Flow

1. **Transition Trigger** → Workflow engine detects available transition
2. **Agent Check** → If agent_execution is configured, start agent
3. **Input Mapping** → Extract data from token using input_mapping
4. **Agent Execution** → Run agent with mapped input data
5. **Streaming** → Broadcast real-time updates to subscribers
6. **Output Mapping** → Apply agent output to token using output_mapping
7. **Transition Complete** → Move token to target place

### Example Configuration

```rust
// Document review workflow with AI agent using Anthropic
let review_transition = TransitionDefinition {
    id: TransitionId::from("ai_review"),
    from_places: vec![PlaceId::from("submitted")],
    to_place: PlaceId::from("reviewed"),
    conditions: vec![],
    rules: vec![],
    agent_execution: Some(AgentTransitionConfig {
        agent_id: AgentId::from("content-reviewer"),
        input_mapping: hashmap! {
            "document_content" => "data.content",
            "document_type" => "metadata.type",
            "priority" => "metadata.priority",
        },
        output_mapping: hashmap! {
            "data.review_result" => "assessment",
            "data.review_score" => "confidence_score",
            "metadata.reviewer" => "agent_id",
            "metadata.review_timestamp" => "timestamp",
        },
        required: true,
        timeout_seconds: Some(300),
        retry_config: Some(AgentRetryConfig {
            max_attempts: 3,
            backoff_seconds: 10,
            retry_on_errors: vec!["timeout".to_string(), "rate_limit".to_string()],
        }),
    }),
};
```

## Places AI Agent

### Overview

Places AI Agent extends the agent capabilities by allowing AI agents to be automatically executed when tokens are present in specific places within a workflow. This feature builds on the existing agent-enabled transitions but adds the ability to run agents directly against tokens in a place, regardless of transition state.

When configuring a Places AI Agent, you'll need both:
1. The agent configuration (including LLM settings) from an existing AgentDefinition
2. The place-specific configuration that defines when and how to run the agent

### Configuration

```rust
pub struct PlaceAgentConfig {
    pub place_id: PlaceId,                         // Place to monitor
    pub agent_id: AgentId,                         // Agent to run
    pub llm_config: Option<LLMConfig>,             // Override default LLM settings
    pub trigger_conditions: Vec<Rule>,             // Optional conditions for triggering
    pub input_mapping: HashMap<String, String>,    // Map token data to agent input
    pub output_mapping: HashMap<String, String>,   // Map agent output to token
    pub auto_transition: Option<TransitionId>,     // Optional transition to fire after completion
    pub schedule: Option<PlaceAgentSchedule>,      // Optional scheduling parameters
    pub retry_config: Option<AgentRetryConfig>,    // Retry configuration
}

pub struct PlaceAgentSchedule {
    pub initial_delay_seconds: Option<u64>,        // Delay before first execution
    pub interval_seconds: Option<u64>,             // Periodic execution interval
    pub max_executions: Option<u32>,               // Maximum number of executions
}
```

### Execution Flow

1. **Token Placed** → Token enters or exists in monitored place
2. **Condition Check** → Evaluate trigger conditions (if any)
3. **Scheduling** → Apply scheduling constraints (if configured)
4. **Input Mapping** → Extract data from token using input_mapping
5. **Agent Execution** → Run agent with mapped input data
6. **Streaming** → Broadcast real-time updates to subscribers
7. **Output Mapping** → Apply agent output to token using output_mapping
8. **Auto Transition** → Optionally trigger a transition after completion

### Example Configuration

```rust
// Places AI Agent for content classification using Anthropic
let classification_agent = PlaceAgentConfig {
    place_id: PlaceId::from("pending_classification"),
    agent_id: AgentId::from("content-classifier"),
    llm_config: Some(LLMConfig {
        temperature: 0.1,                        // Very low temperature for consistent classification
        max_tokens: 200,                         // Limit response size for classification
        top_p: 0.9,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        stop_sequences: vec!["CLASSIFICATION COMPLETE".to_string()],
    }),
    trigger_conditions: vec![
        Rule::field_exists("data", "content"),
        Rule::field_equals("metadata.status", "unclassified"),
    ],
    input_mapping: hashmap! {
        "content" => "data.content",
        "content_type" => "metadata.type",
    },
    output_mapping: hashmap! {
        "data.classification" => "category",
        "data.confidence" => "confidence_score",
        "metadata.classifier" => "agent_id",
        "metadata.classified_at" => "timestamp",
    },
    auto_transition: Some(TransitionId::from("move_to_categorized")),
    schedule: Some(PlaceAgentSchedule {
        initial_delay_seconds: Some(5),
        interval_seconds: None,
        max_executions: Some(1),
    }),
    retry_config: Some(AgentRetryConfig {
        max_attempts: 2,
        backoff_seconds: 15,
        retry_on_errors: vec!["timeout".to_string(), "rate_limit".to_string()],
    }),
};
```

### Use Cases

#### 1. Background Processing
Run agents on tokens in specific places without requiring user-initiated transitions.

#### 2. Periodic Analysis
Schedule agents to periodically analyze or update tokens that remain in a place for extended periods.

#### 3. Cascading Agent Workflows
Create chains of agent processing by using auto-transitions to move tokens between agent-enabled places.

#### 4. Conditional Processing
Apply complex business rules to determine when agents should be triggered in a place.

#### 5. LLM Parameter Optimization
Override default LLM settings for specific places to optimize for different tasks:
- Lower temperature (0.1-0.2) for classification/analysis tasks with Anthropic
- Higher temperature (0.7-1.0) for creative content generation
- Custom stop sequences for place-specific completions
- Model selection per task (claude-3-haiku for speed, claude-3-5-sonnet for quality)

## Streaming Architecture

### Multi-Protocol Support

The streaming system supports multiple protocols for maximum flexibility:

#### 1. GraphQL Subscriptions (Primary)
- Built on WebSockets
- Type-safe with schema validation
- Supports complex filtering
- Integrates with existing GraphQL API

#### 2. Direct WebSocket (High Performance)
- Lower latency
- Simple message format
- Better for high-frequency updates
- Custom connection handling

#### 3. Server-Sent Events (Optional)
- HTTP-based streaming
- Firewall-friendly
- Browser-compatible
- Automatic reconnection

### Stream Event Types

```rust
pub enum AgentStreamEvent {
    ContentChunk {
        execution_id: String,
        chunk: String,
        sequence: u32,
    },
    ThinkingStatus {
        execution_id: String,
        status: String,
    },
    ToolCall {
        execution_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        execution_id: String,
        tool_name: String,
        result: serde_json::Value,
    },
    Completed {
        execution_id: String,
        final_response: String,
        usage: TokenUsage,
    },
    Failed {
        execution_id: String,
        error: String,
    },
}
```

### Client Usage Examples

#### GraphQL Subscription (TypeScript)
```typescript
const AGENT_STREAM_SUBSCRIPTION = gql`
  subscription AgentExecutionStream($executionId: String!) {
    agentExecutionStream(executionId: $executionId) {
      executionId
      eventType
      content
      status
      toolName
      error
      timestamp
    }
  }
`;

const { data, loading, error } = useSubscription(AGENT_STREAM_SUBSCRIPTION, {
  variables: { executionId: "abc-123" }
});
```

#### Direct WebSocket (JavaScript)
```javascript
const ws = new WebSocket('ws://localhost:4000/agent-stream/abc-123');

ws.onmessage = (event) => {
  const agentEvent = JSON.parse(event.data);

  switch (agentEvent.eventType) {
    case 'content_chunk':
      appendToResponse(agentEvent.content);
      break;
    case 'thinking':
      showThinkingIndicator(agentEvent.status);
      break;
    case 'completed':
      hideThinkingIndicator();
      showFinalResponse(agentEvent.final_response);
      break;
    case 'failed':
      showError(agentEvent.error);
      break;
  }
};
```

#### Server-Sent Events (JavaScript)
```javascript
const eventSource = new EventSource('/agent-stream/abc-123');

eventSource.onmessage = (event) => {
  const agentEvent = JSON.parse(event.data);
  handleAgentEvent(agentEvent);
};

eventSource.onerror = (error) => {
  console.error('SSE error:', error);
  // Automatically reconnects
};
```

### Rust Client Example
```rust
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};

async fn stream_agent_response(execution_id: &str) -> Result<()> {
    let url = format!("ws://localhost:4000/agent-stream/{}", execution_id);
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let event: AgentStreamEvent = serde_json::from_str(&text)?;
                match event {
                    AgentStreamEvent::ContentChunk { chunk, .. } => {
                        print!("{}", chunk);
                        std::io::stdout().flush()?;
                    }
                    AgentStreamEvent::Completed { final_response, .. } => {
                        println!("\n\nFinal response: {}", final_response);
                        break;
                    }
                    AgentStreamEvent::Failed { error, .. } => {
                        eprintln!("Agent failed: {}", error);
                        break;
                    }
                    _ => {}
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}
```

## Storage Requirements

### Agent Storage Schema

```rust
trait AgentStorage {
    // Agent definitions
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()>;
    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>>;
    async fn list_agents(&self) -> Result<Vec<AgentDefinition>>;
    async fn delete_agent(&self, id: &AgentId) -> Result<bool>;

    // Conversations
    async fn store_conversation(&self, conversation: &Conversation) -> Result<()>;
    async fn get_conversation(&self, id: &str) -> Result<Option<Conversation>>;
    async fn list_conversations_for_agent(&self, agent_id: &AgentId) -> Result<Vec<Conversation>>;

    // Executions
    async fn store_execution(&self, execution: &AgentExecution) -> Result<()>;
    async fn get_execution(&self, id: &str) -> Result<Option<AgentExecution>>;
    async fn list_executions_for_token(&self, token_id: &str) -> Result<Vec<AgentExecution>>;
}
```

### Storage Implementations

- **InMemoryAgentStorage**: Development and testing
- **NATSAgentStorage**: Distributed persistence (planned)
- **PostgreSQLAgentStorage**: Relational database (planned)
- **RedisAgentStorage**: Fast caching layer (planned)

## Security Considerations

### API Key Management

```rust
// Environment-based configuration
pub struct SecureAgentConfig {
    pub agent_id: AgentId,
    pub provider_type: String,
    pub model: String,
    pub config: LLMConfig,
    // API keys loaded from environment, not stored
}

impl SecureAgentConfig {
    pub fn load_provider(&self) -> Result<LLMProvider> {
        match self.provider_type.as_str() {
            "openai" => Ok(LLMProvider::OpenAI {
                api_key: std::env::var("OPENAI_API_KEY")?,
                model: self.model.clone(),
                base_url: std::env::var("OPENAI_BASE_URL").ok(),
            }),
            "anthropic" => Ok(LLMProvider::Anthropic {
                api_key: std::env::var("ANTHROPIC_API_KEY")?,
                model: self.model.clone(),
            }),
            // ... other providers
        }
    }
}
```

### Access Control

- **Agent Execution Permissions**: Control which workflows can use which agents
- **Stream Access Control**: Authenticate subscription access
- **Rate Limiting**: Prevent abuse of LLM providers
- **Audit Logging**: Track all agent executions and API calls

### Environment Variables

```bash
# LLM Provider API Keys
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...

# Optional: Custom endpoints
OPENAI_BASE_URL=https://api.openai.com/v1
OLLAMA_BASE_URL=http://localhost:11434

# Security
AGENT_ACCESS_TOKEN=...
STREAM_AUTH_SECRET=...

# Rate Limiting
MAX_CONCURRENT_EXECUTIONS=10
RATE_LIMIT_PER_MINUTE=100
```

## Performance Considerations

### Streaming Optimization

- **Buffer Management**: Configurable buffer sizes for different clients
- **Connection Pooling**: Reuse HTTP connections to LLM providers
- **Backpressure Handling**: Graceful degradation under load
- **Memory Management**: Cleanup completed streams

### LLM Provider Optimization

- **Request Batching**: Combine multiple requests where possible
- **Caching**: Cache responses for identical inputs
- **Fallback Providers**: Switch providers on failures
- **Load Balancing**: Distribute requests across multiple API keys

### Configuration Examples

```rust
pub struct AgentEngineConfig {
    pub max_concurrent_executions: usize,
    pub stream_buffer_size: usize,
    pub connection_timeout: Duration,
    pub execution_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl Default for AgentEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 50,
            stream_buffer_size: 1000,
            connection_timeout: Duration::from_secs(30),
            execution_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}
```

## Testing Strategy

### Unit Tests
- LLM provider implementations
- Stream event parsing
- Agent configuration validation
- Template rendering

### Integration Tests
- End-to-end agent execution
- Multi-protocol streaming
- Workflow integration
- Error handling and retries

### Mock Providers
```rust
pub struct MockLLMProvider {
    responses: Vec<String>,
    delay: Duration,
}

impl MockLLMProvider {
    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            delay: Duration::from_millis(100),
        }
    }
}
```

## Migration and Deployment

### Gradual Rollout
1. Deploy agent infrastructure without workflow integration
2. Test individual agent executions
3. Enable agent transitions for specific workflows
4. Monitor performance and scale as needed

### Monitoring
- Agent execution metrics
- LLM provider response times
- Stream connection counts
- Error rates and types

### Alerting
- Failed agent executions
- High LLM provider latency
- Stream connection issues
- Rate limit exceeded

This architecture provides a robust, scalable foundation for AI agent integration within the Circuit Breaker workflow system, supporting multiple LLM providers with real-time streaming capabilities.
