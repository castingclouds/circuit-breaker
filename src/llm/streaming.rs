//! Streaming Architecture for LLM Router
//! 
//! This module provides multi-protocol streaming support including SSE, WebSocket,
//! and GraphQL subscriptions for real-time LLM response streaming.

use super::*;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

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
        final_data: Option<serde_json::Value>,
    },
    #[serde(rename = "usage")]
    Usage {
        id: String,
        tokens_used: u32,
        cost: f64,
    },
}

/// Flow control for adaptive streaming
#[derive(Debug, Clone)]
pub struct FlowControl {
    pub max_rate: f64,          // Items per second
    pub burst_size: usize,      // Maximum burst items
    last_sent: Instant,
    tokens: f64,
    max_tokens: f64,
}

impl FlowControl {
    pub fn new(max_rate: f64, burst_size: usize) -> Self {
        Self {
            max_rate,
            burst_size,
            last_sent: Instant::now(),
            tokens: burst_size as f64,
            max_tokens: burst_size as f64,
        }
    }

    pub fn should_send(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sent).as_secs_f64();
        
        // Refill tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * self.max_rate).min(self.max_tokens);
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.last_sent = now;
            true
        } else {
            false
        }
    }

    pub fn on_item_sent(&mut self) {
        // This can be used for additional bookkeeping if needed
    }
}

/// High-level streaming manager for LLM responses
#[derive(Debug)]
pub struct StreamingManager {
    pub active_sessions: Arc<RwLock<HashMap<Uuid, StreamingSession>>>,
    pub active_streams: Arc<RwLock<HashMap<Uuid, mpsc::Sender<StreamEvent>>>>,
    pub config: StreamingConfig,
}

#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub max_concurrent_streams: usize,
    pub default_buffer_size: usize,
    pub session_timeout: Duration,
    pub max_chunk_size: usize,
    pub enable_flow_control: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 1000,
            default_buffer_size: 100,
            session_timeout: Duration::from_secs(300), // 5 minutes
            max_chunk_size: 8192,
            enable_flow_control: true,
        }
    }
}

impl StreamingManager {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn create_session(
        &self,
        protocol: StreamingProtocol,
        user_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4();
        let session = StreamingSession {
            id: session_id,
            protocol,
            user_id,
            project_id,
            started_at: Utc::now(),
            last_activity: Utc::now(),
        };

        let mut sessions = self.active_sessions.write().await;
        if sessions.len() >= self.config.max_concurrent_streams {
            return Err("Maximum concurrent streams reached".into());
        }

        sessions.insert(session_id, session);
        Ok(session_id)
    }

    pub async fn start_stream(
        &self,
        session_id: Uuid,
        stream: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
    ) -> Result<mpsc::Receiver<StreamEvent>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(self.config.default_buffer_size);

        {
            let mut streams = self.active_streams.write().await;
            streams.insert(session_id, tx.clone());
        }

        // Spawn task to handle the stream
        let tx_clone = tx.clone();
        let streams_arc = self.active_streams.clone();
        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(event) = stream.next().await {
                if tx_clone.send(event).await.is_err() {
                    break;
                }
            }
            
            // Clean up when stream ends
            let mut streams = streams_arc.write().await;
            streams.remove(&session_id);
        });

        Ok(rx)
    }

    pub async fn close_session(&self, session_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.active_sessions.write().await;
        let mut streams = self.active_streams.write().await;
        
        sessions.remove(&session_id);
        if let Some(sender) = streams.remove(&session_id) {
            // Sender will be dropped, closing the channel
            drop(sender);
        }
        
        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let timeout = self.config.session_timeout;
        let now = Utc::now();
        
        let mut sessions = self.active_sessions.write().await;
        let mut streams = self.active_streams.write().await;
        
        let expired_sessions: Vec<Uuid> = sessions
            .iter()
            .filter_map(|(id, session)| {
                if now.signed_duration_since(session.last_activity).to_std().unwrap_or(Duration::ZERO) > timeout {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        for session_id in expired_sessions {
            sessions.remove(&session_id);
            streams.remove(&session_id);
        }
    }

    pub async fn get_active_session_count(&self) -> usize {
        let sessions = self.active_sessions.read().await;
        sessions.len()
    }
}

/// Convert LLM streaming chunks to StreamEvent
impl From<StreamingChunk> for StreamEvent {
    fn from(chunk: StreamingChunk) -> Self {
        StreamEvent::Chunk {
            id: chunk.id.clone(),
            data: chunk,
        }
    }
}

/// Utility function to create a simple streaming chunk
pub fn create_streaming_chunk(
    id: String,
    content: String,
    model: String,
    provider: LLMProviderType,
    finish_reason: Option<String>,
) -> StreamingChunk {
    StreamingChunk {
        id,
        object: "chat.completion.chunk".to_string(),
        choices: vec![StreamingChoice {
            index: 0,
            delta: ChatMessage {
                role: MessageRole::Assistant,
                content,
                name: None,
                function_call: None,
            },
            finish_reason,
        }],
        created: chrono::Utc::now().timestamp() as u64,
        model,
        provider,
    }
}

/// Create an error stream event
pub fn create_error_event(id: String, error: String, code: Option<String>) -> StreamEvent {
    StreamEvent::Error { id, error, code }
}

/// Create a completion done event
pub fn create_done_event(id: String, final_data: Option<serde_json::Value>) -> StreamEvent {
    StreamEvent::Done { id, final_data }
}

/// Create a usage tracking event
pub fn create_usage_event(id: String, tokens_used: u32, cost: f64) -> StreamEvent {
    StreamEvent::Usage { id, tokens_used, cost }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_streaming_manager_creation() {
        let config = StreamingConfig::default();
        let manager = StreamingManager::new(config);
        
        assert_eq!(manager.get_active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_creation() {
        let manager = StreamingManager::new(StreamingConfig::default());
        
        let session_id = manager
            .create_session(StreamingProtocol::ServerSentEvents, None, None)
            .await
            .unwrap();
        
        assert_eq!(manager.get_active_session_count().await, 1);
        
        manager.close_session(session_id).await.unwrap();
        assert_eq!(manager.get_active_session_count().await, 0);
    }

    #[test]
    fn test_flow_control() {
        let mut flow_control = FlowControl::new(10.0, 5); // 10 items/sec, burst of 5
        
        // Should allow initial burst
        for _ in 0..5 {
            assert!(flow_control.should_send());
        }
        
        // Should block after burst
        assert!(!flow_control.should_send());
    }

    #[test]
    fn test_streaming_chunk_conversion() {
        let chunk = create_streaming_chunk(
            "test-id".to_string(),
            "Hello, world!".to_string(),
            "gpt-4".to_string(),
            LLMProviderType::OpenAI,
            None,
        );
        
        let event: StreamEvent = chunk.into();
        
        match event {
            StreamEvent::Chunk { id, data } => {
                assert_eq!(id, "test-id");
                assert_eq!(data.choices[0].delta.content, "Hello, world!");
            }
            _ => panic!("Expected chunk event"),
        }
    }
}