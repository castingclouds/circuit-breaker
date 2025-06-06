//! Streaming Architecture for LLM Router
//! 
//! This module provides multi-protocol streaming support including SSE, WebSocket,
//! and GraphQL subscriptions for real-time LLM response streaming.

use super::*;
use futures::{Stream, StreamExt, SinkExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use serde_json::json;
use uuid::Uuid;
use std::collections::HashMap;

/// Streaming protocol types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamingProtocol {
    ServerSentEvents,
    WebSocket,
    GraphQLSubscription,
}

/// Streaming session information
#[derive(Debug, Clone)]
pub struct StreamingSession {
    pub id: Uuid,
    pub protocol: StreamingProtocol,
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Stream event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "chunk")]
    Chunk {
        id: String,
        data: StreamingChunk,
    },
    #[serde(rename = "error")]
    Error {
        id: String,
        error: String,
        code: Option<String>,
    },
    #[serde(rename = "done")]
    Done {
        id: String,
        usage: Option<TokenUsage>,
        final_response: Option<LLMResponse>,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat {
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "metadata")]
    Metadata {
        id: String,
        routing_info: RoutingInfo,
        cost_info: Option<CostInfo>,
    },
}

/// Streaming manager that handles multiple protocols
pub struct StreamingManager {
    active_sessions: Arc<RwLock<HashMap<Uuid, StreamingSession>>>,
    session_channels: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<StreamEvent>>>>,
    heartbeat_interval: Duration,
}

impl StreamingManager {
    pub fn new() -> Self {
        let manager = Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            session_channels: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_interval: Duration::from_secs(30),
        };
        
        // Start heartbeat task
        manager.start_heartbeat_task();
        
        manager
    }

    /// Create a new streaming session
    pub async fn create_session(
        &self,
        protocol: StreamingProtocol,
        user_id: Option<String>,
        project_id: Option<String>,
    ) -> (Uuid, mpsc::UnboundedReceiver<StreamEvent>) {
        let session_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        
        let session = StreamingSession {
            id: session_id,
            protocol,
            user_id,
            project_id,
            started_at: Utc::now(),
            last_activity: Utc::now(),
        };
        
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, session);
        }
        
        {
            let mut channels = self.session_channels.write().await;
            channels.insert(session_id, tx);
        }
        
        (session_id, rx)
    }

    /// Send event to a specific session
    pub async fn send_to_session(&self, session_id: Uuid, event: StreamEvent) -> Result<(), String> {
        let channels = self.session_channels.read().await;
        if let Some(tx) = channels.get(&session_id) {
            tx.send(event).map_err(|e| format!("Failed to send event: {}", e))?;
            
            // Update last activity
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.last_activity = Utc::now();
            }
            
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id))
        }
    }

    /// Broadcast event to all sessions
    pub async fn broadcast(&self, event: StreamEvent) {
        let channels = self.session_channels.read().await;
        for tx in channels.values() {
            let _ = tx.send(event.clone());
        }
    }

    /// Broadcast to sessions with specific criteria
    pub async fn broadcast_filtered<F>(&self, event: StreamEvent, filter: F)
    where
        F: Fn(&StreamingSession) -> bool,
    {
        let sessions = self.active_sessions.read().await;
        let channels = self.session_channels.read().await;
        
        for (session_id, session) in sessions.iter() {
            if filter(session) {
                if let Some(tx) = channels.get(session_id) {
                    let _ = tx.send(event.clone());
                }
            }
        }
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: Uuid) {
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id);
        }
        
        {
            let mut channels = self.session_channels.write().await;
            channels.remove(&session_id);
        }
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.active_sessions.read().await;
        sessions.len()
    }

    /// Get sessions by protocol
    pub async fn get_sessions_by_protocol(&self, protocol: StreamingProtocol) -> Vec<StreamingSession> {
        let sessions = self.active_sessions.read().await;
        sessions.values()
            .filter(|session| session.protocol == protocol)
            .cloned()
            .collect()
    }

    /// Clean up inactive sessions
    pub async fn cleanup_inactive_sessions(&self, max_idle_duration: Duration) {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(max_idle_duration).unwrap();
        let mut sessions_to_remove = Vec::new();
        
        {
            let sessions = self.active_sessions.read().await;
            for (session_id, session) in sessions.iter() {
                if session.last_activity < cutoff_time {
                    sessions_to_remove.push(*session_id);
                }
            }
        }
        
        for session_id in sessions_to_remove {
            self.remove_session(session_id).await;
        }
    }

    /// Start heartbeat task
    fn start_heartbeat_task(&self) {
        let sessions = self.active_sessions.clone();
        let channels = self.session_channels.clone();
        let interval = self.heartbeat_interval;
        
        tokio::spawn(async move {
            let mut heartbeat_interval = tokio::time::interval(interval);
            
            loop {
                heartbeat_interval.tick().await;
                
                let heartbeat_event = StreamEvent::Heartbeat {
                    timestamp: Utc::now(),
                };
                
                let channels = channels.read().await;
                for tx in channels.values() {
                    let _ = tx.send(heartbeat_event.clone());
                }
            }
        });
    }
}

/// Server-Sent Events (SSE) formatter
pub struct SSEFormatter;

impl SSEFormatter {
    pub fn format_event(event: &StreamEvent) -> String {
        match event {
            StreamEvent::Chunk { id, data } => {
                format!(
                    "id: {}\nevent: chunk\ndata: {}\n\n",
                    id,
                    serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string())
                )
            }
            StreamEvent::Error { id, error, code } => {
                let error_data = json!({
                    "error": error,
                    "code": code
                });
                format!(
                    "id: {}\nevent: error\ndata: {}\n\n",
                    id,
                    serde_json::to_string(&error_data).unwrap_or_else(|_| "{}".to_string())
                )
            }
            StreamEvent::Done { id, usage, final_response } => {
                let done_data = json!({
                    "usage": usage,
                    "final_response": final_response
                });
                format!(
                    "id: {}\nevent: done\ndata: {}\n\n",
                    id,
                    serde_json::to_string(&done_data).unwrap_or_else(|_| "{}".to_string())
                )
            }
            StreamEvent::Heartbeat { timestamp } => {
                format!(
                    "event: heartbeat\ndata: {}\n\n",
                    serde_json::to_string(&json!({ "timestamp": timestamp }))
                        .unwrap_or_else(|_| "{}".to_string())
                )
            }
            StreamEvent::Metadata { id, routing_info, cost_info } => {
                let metadata = json!({
                    "routing_info": routing_info,
                    "cost_info": cost_info
                });
                format!(
                    "id: {}\nevent: metadata\ndata: {}\n\n",
                    id,
                    serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".to_string())
                )
            }
        }
    }
}

/// WebSocket message formatter
pub struct WebSocketFormatter;

impl WebSocketFormatter {
    pub fn format_event(event: &StreamEvent) -> String {
        serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string())
    }
}

/// GraphQL subscription formatter
pub struct GraphQLFormatter;

impl GraphQLFormatter {
    pub fn format_event(event: &StreamEvent, subscription_id: &str) -> String {
        let payload = json!({
            "id": subscription_id,
            "type": "data",
            "payload": {
                "data": {
                    "llmStream": event
                }
            }
        });
        
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Adaptive streaming wrapper that handles buffering and flow control
pub struct AdaptiveStream<T> {
    inner: Pin<Box<dyn Stream<Item = T> + Send>>,
    buffer_size: usize,
    buffer: Vec<T>,
    flow_control: FlowControl,
}

impl<T> AdaptiveStream<T> {
    pub fn new(stream: Pin<Box<dyn Stream<Item = T> + Send>>, buffer_size: usize) -> Self {
        Self {
            inner: stream,
            buffer_size,
            buffer: Vec::with_capacity(buffer_size),
            flow_control: FlowControl::new(),
        }
    }
    
    pub fn with_flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }
}

impl<T> Stream for AdaptiveStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check flow control
        if !self.flow_control.should_send() {
            return Poll::Pending;
        }
        
        // Try to fill buffer
        while self.buffer.len() < self.buffer_size {
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    self.buffer.push(item);
                }
                Poll::Ready(None) => break,
                Poll::Pending => break,
            }
        }
        
        // Return buffered item if available
        if !self.buffer.is_empty() {
            let item = self.buffer.remove(0);
            self.flow_control.on_item_sent();
            Poll::Ready(Some(item))
        } else if self.buffer.is_empty() {
            // Check if inner stream is done
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(None) => Poll::Ready(None),
                _ => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}

/// Flow control for streaming
pub struct FlowControl {
    max_rate: u32,         // items per second
    window_size: Duration, // time window
    sent_count: u32,
    window_start: Instant,
}

impl FlowControl {
    pub fn new() -> Self {
        Self {
            max_rate: 100,
            window_size: Duration::from_secs(1),
            sent_count: 0,
            window_start: Instant::now(),
        }
    }
    
    pub fn with_rate(mut self, max_rate: u32) -> Self {
        self.max_rate = max_rate;
        self
    }
    
    pub fn should_send(&mut self) -> bool {
        let now = Instant::now();
        
        // Reset window if needed
        if now.duration_since(self.window_start) >= self.window_size {
            self.sent_count = 0;
            self.window_start = now;
        }
        
        self.sent_count < self.max_rate
    }
    
    pub fn on_item_sent(&mut self) {
        self.sent_count += 1;
    }
}

/// Stream multiplexer for handling multiple concurrent streams
pub struct StreamMultiplexer {
    streams: HashMap<Uuid, Box<dyn Stream<Item = StreamEvent> + Send + Unpin>>,
    output_tx: mpsc::UnboundedSender<(Uuid, StreamEvent)>,
    output_rx: mpsc::UnboundedReceiver<(Uuid, StreamEvent)>,
}

impl StreamMultiplexer {
    pub fn new() -> Self {
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        
        Self {
            streams: HashMap::new(),
            output_tx,
            output_rx,
        }
    }
    
    pub fn add_stream(&mut self, id: Uuid, stream: Box<dyn Stream<Item = StreamEvent> + Send + Unpin>) {
        let tx = self.output_tx.clone();
        
        // Spawn task to forward stream events
        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(event) = stream.next().await {
                if tx.send((id, event)).is_err() {
                    break;
                }
            }
        });
        
        self.streams.insert(id, stream);
    }
    
    pub fn remove_stream(&mut self, id: Uuid) {
        self.streams.remove(&id);
    }
    
    pub async fn next_event(&mut self) -> Option<(Uuid, StreamEvent)> {
        self.output_rx.recv().await
    }
}

/// Streaming response aggregator
pub struct StreamAggregator {
    chunks: Vec<StreamingChunk>,
    current_content: String,
    usage: Option<TokenUsage>,
    routing_info: Option<RoutingInfo>,
}

impl StreamAggregator {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            current_content: String::new(),
            usage: None,
            routing_info: None,
        }
    }
    
    pub fn add_chunk(&mut self, chunk: StreamingChunk) {
        // Aggregate content from delta
        if let Some(choice) = chunk.choices.first() {
            self.current_content.push_str(&choice.delta.content);
        }
        
        self.chunks.push(chunk);
    }
    
    pub fn set_usage(&mut self, usage: TokenUsage) {
        self.usage = Some(usage);
    }
    
    pub fn set_routing_info(&mut self, routing_info: RoutingInfo) {
        self.routing_info = Some(routing_info);
    }
    
    pub fn build_final_response(&self, request_id: String) -> LLMResponse {
        let model = self.chunks.first()
            .map(|c| c.model.clone())
            .unwrap_or_default();
            
        let provider = self.chunks.first()
            .map(|c| c.provider.clone())
            .unwrap_or(LLMProviderType::OpenAI);
        
        LLMResponse {
            id: request_id,
            object: "chat.completion".to_string(),
            created: Utc::now().timestamp() as u64,
            model,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: self.current_content.clone(),
                    name: None,
                    function_call: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: self.usage.clone().unwrap_or_default(),
            provider,
            routing_info: self.routing_info.clone().unwrap_or(RoutingInfo {
                selected_provider: provider,
                routing_strategy: RoutingStrategy::CostOptimized,
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
            }),
        }
    }
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            estimated_cost: 0.0,
        }
    }
}

/// Streaming utilities
pub mod utils {
    use super::*;
    
    /// Convert LLM provider stream to our stream events
    pub fn convert_provider_stream(
        provider_stream: Box<dyn Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>,
        request_id: String,
    ) -> Box<dyn Stream<Item = StreamEvent> + Send + Unpin> {
        let stream = provider_stream.map(move |result| {
            match result {
                Ok(chunk) => StreamEvent::Chunk {
                    id: request_id.clone(),
                    data: chunk,
                },
                Err(e) => StreamEvent::Error {
                    id: request_id.clone(),
                    error: e.to_string(),
                    code: Some("provider_error".to_string()),
                },
            }
        });
        
        Box::new(Box::pin(stream))
    }
    
    /// Create a test stream for development
    pub fn create_test_stream(messages: Vec<String>) -> Box<dyn Stream<Item = StreamEvent> + Send + Unpin> {
        let stream = futures::stream::iter(messages.into_iter().enumerate().map(|(i, content)| {
            StreamEvent::Chunk {
                id: format!("test-{}", i),
                data: StreamingChunk {
                    id: format!("test-{}", i),
                    object: "chat.completion.chunk".to_string(),
                    created: Utc::now().timestamp() as u64,
                    model: "test-model".to_string(),
                    choices: vec![StreamingChoice {
                        index: 0,
                        delta: ChatMessage {
                            role: MessageRole::Assistant,
                            content,
                            name: None,
                            function_call: None,
                        },
                        finish_reason: None,
                    }],
                    provider: LLMProviderType::OpenAI,
                },
            }
        }));
        
        Box::new(Box::pin(stream))
    }
}