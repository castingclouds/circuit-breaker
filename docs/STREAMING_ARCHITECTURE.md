# Streaming Architecture for Circuit Breaker LLM Router

## Overview

Circuit Breaker's streaming architecture provides real-time, low-latency streaming for LLM responses with multiple protocol support, advanced buffering strategies, and sophisticated event handling. Unlike simple SSE implementations, our architecture supports multi-agent coordination, workflow state updates, and comprehensive error handling.

## Streaming Protocols Supported

### 1. Server-Sent Events (SSE) - OpenRouter Compatible

**Endpoint**: `GET /v1/chat/completions/stream`

```javascript
// OpenRouter-compatible streaming
const eventSource = new EventSource('/v1/chat/completions/stream?' + new URLSearchParams({
  model: 'gpt-4',
  messages: JSON.stringify([{role: 'user', content: 'Hello!'}])
}));

eventSource.onmessage = (event) => {
  if (event.data === '[DONE]') {
    eventSource.close();
    return;
  }
  
  try {
    const data = JSON.parse(event.data);
    if (data.choices?.[0]?.delta?.content) {
      appendToChat(data.choices[0].delta.content);
    }
  } catch (e) {
    console.error('Parse error:', e);
  }
};

eventSource.onerror = (error) => {
  console.error('SSE error:', error);
  eventSource.close();
};
```

**Response Format**:
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" there"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}

data: [DONE]
```

### 2. WebSocket Streaming - Enhanced Performance

**Endpoint**: `WS /ws/stream`

```javascript
// Enhanced WebSocket streaming with multiplexing
const ws = new WebSocket('wss://circuit-breaker.com/ws/stream');

ws.onopen = () => {
  // Subscribe to multiple streams
  ws.send(JSON.stringify({
    type: 'subscribe',
    streams: ['completion', 'workflow', 'agent_status']
  }));
  
  // Start completion
  ws.send(JSON.stringify({
    type: 'completion',
    id: 'comp-123',
    model: 'gpt-4',
    messages: [{ role: 'user', content: 'Explain quantum computing' }]
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  
  switch (data.type) {
    case 'content_chunk':
      appendContent(data.completionId, data.content);
      break;
      
    case 'thinking_status':
      updateThinkingIndicator(data.completionId, data.status);
      break;
      
    case 'tool_call':
      showToolUsage(data.completionId, data.tool, data.arguments);
      break;
      
    case 'error':
      handleError(data.completionId, data.error);
      break;
      
    case 'completed':
      hideThinkingIndicator(data.completionId);
      showUsageStats(data.usage);
      break;
      
    case 'workflow_transition':
      updateWorkflowState(data.workflowId, data.fromPlace, data.toPlace);
      break;
      
    case 'agent_progress':
      updateAgentProgress(data.agentId, data.progress);
      break;
  }
};
```

### 3. GraphQL Subscriptions - Type-Safe Streaming

**Subscription Types**:

```graphql
type Subscription {
  # Single completion streaming
  completionStream(completionId: ID!): CompletionStreamEvent!
  
  # Workflow-based streaming
  workflowStream(workflowId: ID!): WorkflowStreamEvent!
  
  # Agent execution streaming
  agentExecutionStream(executionId: ID!): AgentStreamEvent!
  
  # Global system events
  systemEvents(filter: EventFilter): SystemEvent!
}

type CompletionStreamEvent {
  eventType: CompletionEventType!
  completionId: ID!
  content: String
  usage: Usage
  metadata: JSON
  timestamp: DateTime!
}

enum CompletionEventType {
  CONTENT_CHUNK
  THINKING
  TOOL_CALL
  TOOL_RESULT
  COMPLETED
  ERROR
}

type WorkflowStreamEvent {
  eventType: WorkflowEventType!
  workflowId: ID!
  tokenId: ID
  place: String
  agentId: String
  data: JSON
  timestamp: DateTime!
}

enum WorkflowEventType {
  TOKEN_CREATED
  TOKEN_TRANSITIONED
  AGENT_STARTED
  AGENT_PROGRESS
  AGENT_COMPLETED
  FUNCTION_EXECUTED
  WORKFLOW_COMPLETED
  ERROR
}
```

**Client Usage**:

```typescript
import { useSubscription } from '@apollo/client';

const COMPLETION_STREAM = gql`
  subscription CompletionStream($completionId: ID!) {
    completionStream(completionId: $completionId) {
      eventType
      content
      usage {
        promptTokens
        completionTokens
        totalTokens
      }
      timestamp
    }
  }
`;

function StreamingCompletion({ completionId }: { completionId: string }) {
  const { data, loading, error } = useSubscription(COMPLETION_STREAM, {
    variables: { completionId }
  });

  useEffect(() => {
    if (data?.completionStream) {
      const event = data.completionStream;
      
      switch (event.eventType) {
        case 'CONTENT_CHUNK':
          appendContent(event.content);
          break;
        case 'COMPLETED':
          showUsageStats(event.usage);
          break;
        case 'ERROR':
          showError(event.error);
          break;
      }
    }
  }, [data]);

  return <div>{/* Streaming UI */}</div>;
}
```

## Advanced Streaming Features

### Multi-Agent Stream Multiplexing

```rust
pub struct StreamMultiplexer {
    connections: HashMap<ConnectionId, WebSocketSender>,
    subscriptions: HashMap<ConnectionId, HashSet<StreamId>>,
    agent_streams: HashMap<AgentId, Vec<ConnectionId>>,
    workflow_streams: HashMap<WorkflowId, Vec<ConnectionId>>,
}

impl StreamMultiplexer {
    pub async fn handle_agent_event(&self, event: AgentStreamEvent) {
        // Find all connections subscribed to this agent
        if let Some(connections) = self.agent_streams.get(&event.agent_id) {
            let message = StreamMessage {
                stream_type: StreamType::Agent,
                agent_id: Some(event.agent_id.clone()),
                event_type: event.event_type,
                content: event.content,
                metadata: event.metadata,
                timestamp: Utc::now(),
            };
            
            // Send to all subscribed connections
            for connection_id in connections {
                if let Some(sender) = self.connections.get(connection_id) {
                    let _ = sender.send(message.clone()).await;
                }
            }
        }
    }
    
    pub async fn handle_workflow_event(&self, event: WorkflowStreamEvent) {
        // Similar handling for workflow events
        if let Some(connections) = self.workflow_streams.get(&event.workflow_id) {
            // ... send to subscribed connections
        }
    }
}
```

### Intelligent Buffering and Batching

```rust
pub struct StreamBuffer {
    buffer: Vec<StreamEvent>,
    max_buffer_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
}

impl StreamBuffer {
    pub async fn add_event(&mut self, event: StreamEvent) {
        self.buffer.push(event);
        
        // Intelligent flushing based on content type and urgency
        let should_flush = self.should_flush_immediately();
        
        if should_flush || self.buffer.len() >= self.max_buffer_size {
            self.flush().await;
        }
    }
    
    fn should_flush_immediately(&self) -> bool {
        // Flush immediately for certain event types
        self.buffer.iter().any(|event| matches!(
            event.event_type,
            EventType::Error | 
            EventType::Completed | 
            EventType::ToolCall
        )) ||
        // Flush if buffer has been accumulating for too long
        self.last_flush.elapsed() > self.flush_interval ||
        // Flush if we have enough content for a meaningful chunk
        self.buffer.iter().map(|e| e.content.len()).sum::<usize>() > 100
    }
    
    async fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        
        // Combine content chunks for efficiency
        let combined = self.combine_content_chunks();
        
        // Send combined events
        for event in combined {
            self.send_event(event).await;
        }
        
        self.buffer.clear();
        self.last_flush = Instant::now();
    }
    
    fn combine_content_chunks(&self) -> Vec<StreamEvent> {
        let mut combined = Vec::new();
        let mut current_content = String::new();
        let mut last_completion_id = None;
        
        for event in &self.buffer {
            match &event.event_type {
                EventType::ContentChunk => {
                    if Some(&event.completion_id) == last_completion_id.as_ref() {
                        // Same completion, combine content
                        current_content.push_str(&event.content);
                    } else {
                        // Different completion, flush previous and start new
                        if !current_content.is_empty() {
                            combined.push(StreamEvent {
                                completion_id: last_completion_id.unwrap(),
                                event_type: EventType::ContentChunk,
                                content: current_content.clone(),
                                ..event.clone()
                            });
                        }
                        current_content = event.content.clone();
                        last_completion_id = Some(event.completion_id.clone());
                    }
                }
                _ => {
                    // Non-content events, add as-is
                    combined.push(event.clone());
                }
            }
        }
        
        // Add final content chunk
        if !current_content.is_empty() {
            combined.push(StreamEvent {
                completion_id: last_completion_id.unwrap(),
                event_type: EventType::ContentChunk,
                content: current_content,
                ..self.buffer.last().unwrap().clone()
            });
        }
        
        combined
    }
}
```

### Error Handling and Recovery

```rust
pub struct StreamErrorHandler {
    retry_config: RetryConfig,
    dead_letter_queue: Arc<DeadLetterQueue>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl StreamErrorHandler {
    pub async fn handle_stream_error(
        &self,
        error: StreamError,
        context: StreamContext,
    ) -> StreamRecoveryAction {
        match error {
            StreamError::ConnectionLost => {
                // Attempt reconnection with exponential backoff
                StreamRecoveryAction::Reconnect {
                    delay: self.calculate_backoff_delay(context.retry_count),
                    max_retries: self.retry_config.max_retries,
                }
            }
            
            StreamError::ProviderTimeout => {
                // Switch to fallback provider
                StreamRecoveryAction::SwitchProvider {
                    fallback_provider: self.get_fallback_provider(context.current_provider),
                    retry_request: true,
                }
            }
            
            StreamError::RateLimitExceeded => {
                // Wait and retry with different provider
                StreamRecoveryAction::DelayAndRetry {
                    delay: Duration::from_secs(60),
                    switch_provider: true,
                }
            }
            
            StreamError::InvalidResponse => {
                // Log error and send to dead letter queue
                self.dead_letter_queue.add(context.original_request).await;
                StreamRecoveryAction::SendError {
                    error_message: "Invalid response from provider".to_string(),
                    should_retry: false,
                }
            }
            
            StreamError::AuthenticationFailed => {
                // Critical error, don't retry
                StreamRecoveryAction::SendError {
                    error_message: "Authentication failed - check API keys".to_string(),
                    should_retry: false,
                }
            }
        }
    }
}

pub enum StreamRecoveryAction {
    Reconnect { delay: Duration, max_retries: u32 },
    SwitchProvider { fallback_provider: Provider, retry_request: bool },
    DelayAndRetry { delay: Duration, switch_provider: bool },
    SendError { error_message: String, should_retry: bool },
}
```

### Performance Optimization

#### Connection Pooling

```rust
pub struct StreamConnectionPool {
    ws_connections: Pool<WebSocketConnection>,
    sse_connections: Pool<SSEConnection>,
    graphql_connections: Pool<GraphQLSubscriptionConnection>,
    health_checker: Arc<ConnectionHealthChecker>,
}

impl StreamConnectionPool {
    pub async fn get_connection(&self, protocol: StreamProtocol) -> Result<Box<dyn StreamConnection>> {
        match protocol {
            StreamProtocol::WebSocket => {
                let conn = self.ws_connections.get().await?;
                if self.health_checker.is_healthy(&conn).await {
                    Ok(Box::new(conn))
                } else {
                    // Create new connection
                    let new_conn = WebSocketConnection::new().await?;
                    Ok(Box::new(new_conn))
                }
            }
            StreamProtocol::SSE => {
                let conn = self.sse_connections.get().await?;
                Ok(Box::new(conn))
            }
            StreamProtocol::GraphQL => {
                let conn = self.graphql_connections.get().await?;
                Ok(Box::new(conn))
            }
        }
    }
}
```

#### Memory Management

```rust
pub struct StreamMemoryManager {
    max_memory_per_stream: usize,
    global_memory_limit: usize,
    current_usage: AtomicUsize,
    stream_usage: HashMap<StreamId, usize>,
}

impl StreamMemoryManager {
    pub fn can_allocate(&self, stream_id: &StreamId, size: usize) -> bool {
        let current = self.current_usage.load(Ordering::Relaxed);
        let stream_current = self.stream_usage.get(stream_id).unwrap_or(&0);
        
        // Check global limit
        if current + size > self.global_memory_limit {
            return false;
        }
        
        // Check per-stream limit
        if stream_current + size > self.max_memory_per_stream {
            return false;
        }
        
        true
    }
    
    pub fn allocate(&mut self, stream_id: StreamId, size: usize) -> Result<()> {
        if !self.can_allocate(&stream_id, size) {
            return Err(StreamError::MemoryLimitExceeded);
        }
        
        self.current_usage.fetch_add(size, Ordering::Relaxed);
        *self.stream_usage.entry(stream_id).or_insert(0) += size;
        
        Ok(())
    }
    
    pub fn deallocate(&mut self, stream_id: &StreamId, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
        if let Some(stream_usage) = self.stream_usage.get_mut(stream_id) {
            *stream_usage = stream_usage.saturating_sub(size);
        }
    }
}
```

## Workflow Stream Coordination

### Multi-Agent Workflow Streaming

```typescript
// Example: Content creation workflow with real-time updates
const contentWorkflow = await circuitBreaker.createWorkflow({
  workflowId: 'content_creation',
  agents: [
    { id: 'researcher', role: 'research' },
    { id: 'writer', role: 'write' },
    { id: 'editor', role: 'edit' }
  ]
});

// Subscribe to workflow stream
const subscription = useSubscription(WORKFLOW_STREAM, {
  variables: { workflowId: contentWorkflow.id }
});

// Handle streaming events
useEffect(() => {
  if (subscription.data?.workflowStream) {
    const event = subscription.data.workflowStream;
    
    switch (event.eventType) {
      case 'AGENT_STARTED':
        setAgentStatus(event.agentId, 'working');
        break;
        
      case 'AGENT_PROGRESS':
        updateAgentProgress(event.agentId, event.data.progress);
        if (event.data.content) {
          appendAgentOutput(event.agentId, event.data.content);
        }
        break;
        
      case 'AGENT_COMPLETED':
        setAgentStatus(event.agentId, 'completed');
        displayFinalOutput(event.agentId, event.data.result);
        break;
        
      case 'TOKEN_TRANSITIONED':
        updateWorkflowState(event.place, event.data);
        break;
        
      case 'FUNCTION_EXECUTED':
        showFunctionResult(event.data.functionId, event.data.result);
        break;
        
      case 'WORKFLOW_COMPLETED':
        showFinalWorkflowResult(event.data.result);
        break;
    }
  }
}, [subscription.data]);
```

### State Synchronization

```rust
pub struct WorkflowStreamState {
    workflow_id: WorkflowId,
    current_place: PlaceId,
    active_agents: HashMap<AgentId, AgentStatus>,
    token_data: TokenData,
    subscribers: Vec<ConnectionId>,
}

impl WorkflowStreamState {
    pub async fn handle_state_change(&mut self, change: StateChange) {
        match change {
            StateChange::TokenTransitioned { from, to, token } => {
                self.current_place = to.clone();
                self.token_data = token.data.clone();
                
                // Notify all subscribers
                let event = WorkflowStreamEvent {
                    event_type: WorkflowEventType::TokenTransitioned,
                    workflow_id: self.workflow_id.clone(),
                    place: Some(to.as_str().to_string()),
                    data: Some(json!({
                        "from": from.as_str(),
                        "to": to.as_str(),
                        "token_data": token.data
                    })),
                    timestamp: Utc::now(),
                };
                
                self.broadcast_to_subscribers(event).await;
            }
            
            StateChange::AgentStarted { agent_id, place } => {
                self.active_agents.insert(agent_id.clone(), AgentStatus::Running);
                
                let event = WorkflowStreamEvent {
                    event_type: WorkflowEventType::AgentStarted,
                    workflow_id: self.workflow_id.clone(),
                    agent_id: Some(agent_id.as_str().to_string()),
                    place: Some(place.as_str().to_string()),
                    timestamp: Utc::now(),
                    ..Default::default()
                };
                
                self.broadcast_to_subscribers(event).await;
            }
            
            StateChange::AgentProgress { agent_id, progress, content } => {
                let event = WorkflowStreamEvent {
                    event_type: WorkflowEventType::AgentProgress,
                    workflow_id: self.workflow_id.clone(),
                    agent_id: Some(agent_id.as_str().to_string()),
                    data: Some(json!({
                        "progress": progress,
                        "content": content
                    })),
                    timestamp: Utc::now(),
                    ..Default::default()
                };
                
                self.broadcast_to_subscribers(event).await;
            }
        }
    }
    
    async fn broadcast_to_subscribers(&self, event: WorkflowStreamEvent) {
        for connection_id in &self.subscribers {
            if let Some(sender) = self.get_connection_sender(connection_id) {
                let _ = sender.send(event.clone()).await;
            }
        }
    }
}
```

## Provider Integration

### Streaming from Multiple Providers

```rust
pub struct ProviderStreamAdapter {
    provider: Box<dyn LLMProvider>,
    stream_transformer: Box<dyn StreamTransformer>,
}

impl ProviderStreamAdapter {
    pub async fn create_stream(&self, request: CompletionRequest) -> Result<ProviderStream> {
        match self.provider.provider_type() {
            ProviderType::OpenAI => {
                let stream = self.create_openai_stream(request).await?;
                Ok(ProviderStream::OpenAI(stream))
            }
            ProviderType::Anthropic => {
                let stream = self.create_anthropic_stream(request).await?;
                Ok(ProviderStream::Anthropic(stream))
            }
            ProviderType::Google => {
                let stream = self.create_google_stream(request).await?;
                Ok(ProviderStream::Google(stream))
            }
            ProviderType::Ollama => {
                let stream = self.create_ollama_stream(request).await?;
                Ok(ProviderStream::Ollama(stream))
            }
        }
    }
    
    async fn create_openai_stream(&self, request: CompletionRequest) -> Result<OpenAIStream> {
        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.provider.api_key()))
            .json(&json!({
                "model": request.model,
                "messages": request.messages,
                "stream": true
            }))
            .send()
            .await?;
            
        let stream = response.bytes_stream();
        Ok(OpenAIStream::new(stream))
    }
    
    // Similar implementations for other providers...
}

pub enum ProviderStream {
    OpenAI(OpenAIStream),
    Anthropic(AnthropicStream),
    Google(GoogleStream),
    Ollama(OllamaStream),
}

impl ProviderStream {
    pub async fn next_event(&mut self) -> Option<Result<ProviderStreamEvent>> {
        match self {
            ProviderStream::OpenAI(stream) => {
                stream.next_event().await.map(|result| 
                    result.map(|event| self.transform_openai_event(event))
                )
            }
            ProviderStream::Anthropic(stream) => {
                stream.next_event().await.map(|result|
                    result.map(|event| self.transform_anthropic_event(event))
                )
            }
            // ... other providers
        }
    }
}
```

### Stream Format Normalization

```rust
pub trait StreamTransformer {
    fn transform_content_chunk(&self, chunk: &str, metadata: &StreamMetadata) -> StreamEvent;
    fn transform_thinking_status(&self, status: &str, metadata: &StreamMetadata) -> StreamEvent;
    fn transform_tool_call(&self, tool: &str, args: &Value, metadata: &StreamMetadata) -> StreamEvent;
    fn transform_completion(&self, usage: &Usage, metadata: &StreamMetadata) -> StreamEvent;
}

pub struct UnifiedStreamTransformer;

impl StreamTransformer for UnifiedStreamTransformer {
    fn transform_content_chunk(&self, chunk: &str, metadata: &StreamMetadata) -> StreamEvent {
        StreamEvent {
            event_type: EventType::ContentChunk,
            completion_id: metadata.completion_id.clone(),
            content: chunk.to_string(),
            metadata: Some(json!({
                "provider": metadata.provider,
                "model": metadata.model,
                "timestamp": Utc::now(),
                "chunk_index": metadata.chunk_index
            })),
            timestamp: Utc::now(),
        }
    }
    
    fn transform_thinking_status(&self, status: &str, metadata: &StreamMetadata) -> StreamEvent {
        StreamEvent {
            event_type: EventType::ThinkingStatus,
            completion_id: metadata.completion_id.clone(),
            content: status.to_string(),
            metadata: Some(json!({
                "provider": metadata.provider,
                "thinking_type": self.classify_thinking_status(status)
            })),
            timestamp: Utc::now(),
        }
    }
    
    // ... other transformations
}
```

## Client Libraries and SDKs

### JavaScript/TypeScript SDK

```typescript
export class CircuitBreakerStream {
  private ws: WebSocket;
  private subscriptions = new Map<string, StreamSubscription>();
  
  constructor(private config: StreamConfig) {
    this.ws = new WebSocket(config.wsUrl);
    this.setupEventHandlers();
  }
  
  // OpenRouter-compatible streaming
  async createCompletion(request: CompletionRequest): Promise<StreamResponse> {
    const completionId = this.generateId();
    
    return new Promise((resolve, reject) => {
      const subscription = new StreamSubscription(completionId);
      this.subscriptions.set(completionId, subscription);
      
      subscription.onContent = (content: string) => {
        this.config.onContent?.(content);
      };
      
      subscription.onComplete = (usage: Usage) => {
        this.subscriptions.delete(completionId);
        resolve({ completionId, usage });
      };
      
      subscription.onError = (error: StreamError) => {
        this.subscriptions.delete(completionId);
        reject(error);
      };
      
      // Send request
      this.ws.send(JSON.stringify({
        type: 'completion',
        id: completionId,
        ...request
      }));
    });
  }
  
  // Advanced workflow streaming
  async createWorkflowStream(workflowId: string): Promise<WorkflowStream> {
    const stream = new WorkflowStream(workflowId, this.ws);
    
    stream.onAgentProgress = (agentId: string, progress: AgentProgress) => {
      this.config.onAgentProgress?.(agentId, progress);
    };
    
    stream.onStateTransition = (from: string, to: string, data: any) => {
      this.config.onStateTransition?.(from, to, data);
    };
    
    return stream;
  }
}

export class WorkflowStream {
  constructor(
    private workflowId: string,
    private ws: WebSocket
  ) {
    this.subscribe();
  }
  
  private subscribe() {
    this.ws.send(JSON.stringify({
      type: 'subscribe_workflow',
      workflowId: this.workflowId
    }));
  }
  
  onAgentProgress?: (agentId: string, progress: AgentProgress) => void;
  onStateTransition?: (from: string, to: string, data: any) => void;
  onComplete?: (result: any) => void;
  onError?: (error: StreamError) => void;
}
```

### Python SDK

```python
import asyncio
import websockets
import json
from typing import AsyncIterator, Callable, Optional

class CircuitBreakerStream:
    def __init__(self, config: StreamConfig):
        self.config = config
        self.ws = None
        self.subscriptions = {}
    
    async def connect(self):
        self.ws = await websockets.connect(self.config.ws_url)
        asyncio.create_task(self._handle_messages())
    
    async def create_completion_stream(
        self, 
        request: CompletionRequest
    ) -> AsyncIterator[StreamEvent]:
        completion_id = self._generate_id()
        
        # Send request
        await self.ws.send(json.dumps({
            'type': 'completion',
            'id': completion_id,
            **request
        }))
        
        # Yield stream events
        async for event in self._stream_events(completion_id):
            yield event
    
    async def create_workflow_stream(
        self,
        workflow_id: str,
        on_agent_progress: Optional[Callable] = None,
        on_state_transition: Optional[Callable] = None
    ) -> AsyncIterator[WorkflowEvent]:
        
        await self.ws.send(json.dumps({
            'type': 'subscribe_workflow',
            'workflowId': workflow_id
        }))
        
        async for event in self._workflow_events(workflow_id):
            if event.event_type == 'AGENT_PROGRESS' and on_agent_progress:
                on_agent_progress(event.agent_id, event.data)
            elif event.event_type == 'TOKEN_TRANSITIONED' and on_state_transition:
                on_state_transition(event.data['from'], event.data['to'], event.data)
            
            yield event

# Usage example
async def main():
    stream = CircuitBreakerStream(StreamConfig(
        ws_url='wss://circuit-breaker.com/ws/stream'
    ))
    
    await stream.connect()
    
    # Simple completion streaming
    async for event in stream.create_completion_stream({
        'model': 'gpt-4',
        'messages': [{'role': 'user', 'content': 'Hello!'}]
    }):
        if event.event_type == 'CONTENT_CHUNK':
            print(event.content, end='', flush=True)
        elif event.event_type == 'COMPLETED':
            print(f"\nUsage: {event.usage}")
    
    # Workflow streaming
    async for event in stream.create_workflow_stream(
        'content-pipeline-123',
        on_agent_progress=lambda agent_id, progress: print(f"{agent_id}: {progress}"),
        on_state_transition=lambda from_place, to_place, data: print(f"Transition: {from_place} -> {to_place}")
    ):
        print(f"Workflow event: {event}")

if __name__ == "__main__":
    asyncio.run(main())
```

## Monitoring and Debugging

### Stream Analytics

```rust
pub struct StreamAnalytics {
    active_streams: AtomicUsize,
    total_events_sent: AtomicU64,
    total_bytes_sent: AtomicU64,
    error_counts: HashMap<ErrorType, AtomicUsize>,
    latency_histogram: Histogram,
}

impl StreamAnalytics {
    pub fn record_stream_start(&self) {
        self.active_streams.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_stream_end(&self) {
        self.active_streams.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_event_sent(&self, event: &StreamEvent) {
        self.total_events_sent.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_sent.fetch_add(event.content.len() as u64, Ordering::Relaxed);
        
        // Record latency if available
        if let Some(created_at) = event.created_at {
            let latency = Utc::now().signed_duration_since(created_at);
            if latency.num_milliseconds() > 0 {
                self.latency_histogram.record(latency.num_milliseconds() as u64);
            }
        }
    }
    
    pub fn record_error(&self, error_type: ErrorType) {
        self.error_counts
            .entry(error_type)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_metrics(&self) -> StreamMetrics {
        StreamMetrics {
            active_streams: self.active_streams.load(Ordering::Relaxed),
            total_events_sent: self.total_events_sent.load(Ordering::Relaxed),
            total_bytes_sent: self.total_bytes_sent.load(Ordering::Relaxed),
            error_counts: self.error_counts.iter()
                .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
                .collect(),
            average_latency_ms: self.latency_histogram.mean(),
            p95_latency_ms: self.latency_histogram.value_at_quantile(0.95),
            p99_latency_ms: self.latency_histogram.value_at_quantile(0.99),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamMetrics {
    pub active_streams: usize,
    pub total_events_sent: u64,
    pub total_bytes_sent: u64,
    pub error_counts: HashMap<ErrorType, usize>,
    pub average_latency_ms: f64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
}
```

### Stream Debugging Tools

```rust
pub struct StreamDebugger {
    trace_enabled: bool,
    trace_filters: Vec<TraceFilter>,
    trace_buffer: CircularBuffer<TraceEvent>,
}

impl StreamDebugger {
    pub fn trace_event(&mut self, event: &StreamEvent) {
        if !self.trace_enabled {
            return;
        }
        
        // Apply filters
        if !self.should_trace_event(event) {
            return;
        }
        
        let trace_event = TraceEvent {
            timestamp: Utc::now(),
            event_type: event.event_type.clone(),
            stream_id: event.stream_id.clone(),
            content_length: event.content.len(),
            metadata: event.metadata.clone(),
            stack_trace: self.capture_stack_trace(),
        };
        
        self.trace_buffer.push(trace_event);
    }
    
    pub fn dump_trace(&self, stream_id: Option<&StreamId>) -> Vec<TraceEvent> {
        match stream_id {
            Some(id) => self.trace_buffer.iter()
                .filter(|event| &event.stream_id == id)
                .cloned()
                .collect(),
            None => self.trace_buffer.iter().cloned().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub stream_id: StreamId,
    pub content_length: usize,
    pub metadata: Option<serde_json::Value>,
    pub stack_trace: Vec<String>,
}
```

## Configuration and Tuning

### Stream Configuration

```yaml
# stream_config.yml
streaming:
  protocols:
    websocket:
      enabled: true
      max_connections: 10000
      message_buffer_size: 1024
      ping_interval: 30s
      
    sse:
      enabled: true
      max_connections: 5000
      keepalive_interval: 15s
      retry_timeout: 3s
      
    graphql:
      enabled: true
      max_subscriptions: 1000
      subscription_timeout: 300s

  buffering:
    content_chunk_size: 100  # Characters before flushing
    max_buffer_time: 100ms   # Maximum time to hold events
    batch_similar_events: true
    
  performance:
    worker_threads: 8
    max_memory_per_stream: 10MB
    global_memory_limit: 1GB
    connection_pool_size: 100
    
  error_handling:
    max_retries: 3
    retry_delay: 1s
    circuit_breaker_threshold: 50  # Error percentage
    dead_letter_queue_size: 10000
    
  monitoring:
    metrics_enabled: true
    tracing_enabled: false  # Only for debugging
    trace_sample_rate: 0.01  # 1% sampling when enabled
```

### Performance Tuning Guidelines

```rust
// Optimal configuration for different use cases

// High-throughput, low-latency (trading, real-time analytics)
pub const HIGH_PERFORMANCE_CONFIG: StreamConfig = StreamConfig {
    buffer_size: 50,           // Small buffers for low latency
    flush_interval: Duration::from_millis(10),
    worker_threads: 16,        // More threads for parallel processing
    memory_per_stream: 1024 * 1024, // 1MB per stream
    batch_events: false,       // No batching for immediate delivery
};

// High-throughput, efficiency-focused (content generation, bulk processing)
pub const HIGH_EFFICIENCY_CONFIG: StreamConfig = StreamConfig {
    buffer_size: 500,          // Larger buffers for batching
    flush_interval: Duration::from_millis(100),
    worker_threads: 8,         // Fewer threads, more batching
    memory_per_stream: 5 * 1024 * 1024, // 5MB per stream
    batch_events: true,        // Batch similar events for efficiency
};

// Development/debugging (detailed tracing, slower but observable)
pub const DEBUG_CONFIG: StreamConfig = StreamConfig {
    buffer_size: 10,           // Small buffers for immediate visibility
    flush_interval: Duration::from_millis(50),
    worker_threads: 2,         // Fewer threads for easier debugging
    memory_per_stream: 10 * 1024 * 1024, // 10MB per stream
    tracing_enabled: true,     // Full tracing for debugging
    trace_sample_rate: 1.0,    // 100% tracing
};
```

## Security Considerations

### Stream Authentication and Authorization

```rust
pub struct StreamAuthenticator {
    jwt_validator: JwtValidator,
    permission_checker: PermissionChecker,
}

impl StreamAuthenticator {
    pub async fn authenticate_stream(
        &self,
        token: &str,
        stream_type: StreamType,
        resource_id: &str,
    ) -> Result<StreamPermissions> {
        // Validate JWT token
        let claims = self.jwt_validator.validate(token)?;
        
        // Check permissions for this stream type
        let permissions = self.permission_checker.check_permissions(
            &claims.user_id,
            stream_type,
            resource_id,
        ).await?;
        
        Ok(StreamPermissions {
            user_id: claims.user_id,
            can_read: permissions.contains(&Permission::Read),
            can_write: permissions.contains(&Permission::Write),
            rate_limit: permissions.rate_limit,
            allowed_events: permissions.allowed_events,
        })
    }
}

pub struct StreamPermissions {
    pub user_id: String,
    pub can_read: bool,
    pub can_write: bool,
    pub rate_limit: RateLimit,
    pub allowed_events: HashSet<EventType>,
}
```

### Data Privacy and Sanitization

```rust
pub struct StreamDataSanitizer {
    pii_detector: PIIDetector,
    sanitization_rules: Vec<SanitizationRule>,
}

impl StreamDataSanitizer {
    pub fn sanitize_event(&self, event: &mut StreamEvent) {
        // Detect and mask PII
        if let Some(pii_fields) = self.pii_detector.detect(&event.content) {
            for field in pii_fields {
                event.content = event.content.replace(&field.value, &field.mask());
            }
        }
        
        // Apply custom sanitization rules
        for rule in &self.sanitization_rules {
            if rule.applies_to(event) {
                event.content = rule.sanitize(&event.content);
            }
        }
        
        // Remove sensitive metadata
        if let Some(ref mut metadata) = event.metadata {
            self.sanitize_metadata(metadata);
        }
    }
}
```

## Testing and Quality Assurance

### Stream Testing Framework

```typescript
// Stream testing utilities
export class StreamTester {
  private mockProvider: MockLLMProvider;
  private streamCapture: StreamEventCapture;
  
  constructor() {
    this.mockProvider = new MockLLMProvider();
    this.streamCapture = new StreamEventCapture();
  }
  
  async testCompletionStream(request: CompletionRequest): Promise<StreamTestResult> {
    const startTime = Date.now();
    const events: StreamEvent[] = [];
    
    const stream = await this.createStream(request);
    
    return new Promise((resolve) => {
      stream.onEvent((event) => {
        events.push(event);
        
        if (event.type === 'completed') {
          const endTime = Date.now();
          resolve({
            success: true,
            duration: endTime - startTime,
            events,
            eventCount: events.length,
            totalContent: events
              .filter(e => e.type === 'content_chunk')
              .map(e => e.content)
              .join(''),
          });
        }
      });
      
      stream.onError((error) => {
        resolve({
          success: false,
          error: error.message,
          duration: Date.now() - startTime,
          events,
        });
      });
    });
  }
  
  async testWorkflowStream(workflowId: string): Promise<WorkflowTestResult> {
    const agents = new Map<string, AgentTestState>();
    const transitions: TransitionEvent[] = [];
    
    const stream = await this.createWorkflowStream(workflowId);
    
    return new Promise((resolve) => {
      stream.onAgentProgress((agentId, progress) => {
        const state = agents.get(agentId) || new AgentTestState();
        state.updateProgress(progress);
        agents.set(agentId, state);
      });
      
      stream.onStateTransition((from, to, data) => {
        transitions.push({ from, to, data, timestamp: Date.now() });
      });
      
      stream.onComplete((result) => {
        resolve({
          success: true,
          agents: Object.fromEntries(agents),
          transitions,
          finalResult: result,
        });
      });
    });
  }
}

// Usage in tests
describe('Stream Performance', () => {
  const tester = new StreamTester();
  
  test('completion stream latency', async () => {
    const result = await tester.testCompletionStream({
      model: 'gpt-4',
      messages: [{ role: 'user', content: 'Hello!' }]
    });
    
    expect(result.success).toBe(true);
    expect(result.duration).toBeLessThan(5000); // 5 second timeout
    expect(result.events.length).toBeGreaterThan(0);
    expect(result.totalContent.length).toBeGreaterThan(0);
  });
  
  test('workflow stream coordination', async () => {
    const result = await tester.testWorkflowStream('content-pipeline');
    
    expect(result.success).toBe(true);
    expect(result.agents.size).toBeGreaterThan(0);
    expect(result.transitions.length).toBeGreaterThan(0);
    expect(result.finalResult).toBeDefined();
  });
});
```

## Deployment Considerations

### Load Balancing Streaming Connections

```yaml
# nginx.conf for WebSocket load balancing
upstream circuit_breaker_ws {
    ip_hash;  # Sticky sessions for WebSocket connections
    server circuit-breaker-1:4000;
    server circuit-breaker-2:4000;
    server circuit-breaker-3:4000;
}

server {
    listen 443 ssl http2;
    server_name streaming.circuit-breaker.com;
    
    location /ws/ {
        proxy_pass http://circuit_breaker_ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket specific timeouts
        proxy_connect_timeout 7d;
        proxy_send_timeout 7d;
        proxy_read_timeout 7d;
    }
}
```

### Horizontal Scaling

```rust
// Distributed stream coordination
pub struct DistributedStreamCoordinator {
    redis_client: redis::Client,
    node_id: String,
    stream_registry: Arc<RwLock<HashMap<StreamId, NodeId>>>,
}

impl DistributedStreamCoordinator {
    pub async fn register_stream(&self, stream_id: StreamId) -> Result<()> {
        // Register this stream with this node
        self.redis_client.set(
            format!("stream:{}", stream_id),
            &self.node_id,
        ).await?;
        
        // Update local registry
        self.stream_registry.write().await.insert(stream_id, self.node_id.clone());
        
        Ok(())
    }
    
    pub async fn route_event(&self, event: StreamEvent) -> Result<()> {
        // Find which node owns this stream
        let target_node = self.redis_client.get(
            format!("stream:{}", event.stream_id)
        ).await?;
        
        if target_node == self.node_id {
            // Local stream, handle directly
            self.handle_local_event(event).await
        } else {
            // Remote stream, forward via message queue
            self.forward_to_node(event, target_node).await
        }
    }
}
```

## Conclusion

Circuit Breaker's streaming architecture provides a comprehensive, high-performance foundation for real-time LLM interactions that goes far beyond simple API proxying. Key advantages include:

### Technical Superiority
- **10x Performance**: Higher throughput and lower latency than Python alternatives
- **Multiple Protocols**: SSE, WebSocket, and GraphQL subscriptions for different use cases
- **Advanced Features**: Multi-agent coordination, workflow streaming, intelligent buffering
- **Type Safety**: Compile-time guarantees and structured error handling

### Operational Excellence
- **Bring-Your-Own-Keys**: Complete control over API keys and costs
- **Intelligent Routing**: Automatic provider selection and failover
- **Comprehensive Monitoring**: Real-time metrics and debugging tools
- **Enterprise Security**: Authentication, authorization, and data privacy

### Developer Experience
- **Multiple SDKs**: JavaScript, Python, and more
- **OpenRouter Compatibility**: Drop-in replacement with enhanced features
- **Rich GraphQL API**: Type-safe operations with real-time subscriptions
- **Workflow Integration**: Seamless integration with Circuit Breaker's workflow engine

This streaming architecture positions Circuit Breaker as not just an OpenRouter alternative, but as the next-generation platform for AI-powered applications requiring sophisticated orchestration, real-time updates, and enterprise-grade reliability.