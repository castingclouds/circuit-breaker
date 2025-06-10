//! Server-Sent Events (SSE) parsing utilities for LLM streaming
//! 
//! This module provides utilities for parsing SSE streams from different LLM providers.
//! Each provider has slightly different SSE formats, so we provide both generic and
//! provider-specific parsing functions.

use futures::{Stream, StreamExt};
use tracing::{debug, error};

use crate::llm::{LLMError, LLMResult, StreamingChunk, StreamingChoice, ChatMessage, MessageRole, LLMProviderType};

/// SSE event structure
#[derive(Debug, Clone)]
pub struct SSEEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u32>,
}

/// SSE stream parser that converts bytes into SSE events
pub struct SSEParser {
    buffer: String,
}

impl SSEParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse bytes into SSE events
    pub fn parse_chunk(&mut self, chunk: &[u8]) -> LLMResult<Vec<SSEEvent>> {
        let chunk_str = std::str::from_utf8(chunk)
            .map_err(|e| LLMError::Parse(format!("Invalid UTF-8 in SSE stream: {}", e)))?;
        
        self.buffer.push_str(chunk_str);
        
        let mut events = Vec::new();
        
        // Split buffer by double newlines (event boundaries)
        while let Some(double_newline_pos) = self.buffer.find("\n\n") {
            let event_block = self.buffer[..double_newline_pos].to_string();
            self.buffer = self.buffer[double_newline_pos + 2..].to_string();
            
            debug!("Found event block: {:?}", event_block);
            
            if !event_block.trim().is_empty() {
                match self.parse_event_block(&event_block) {
                    Ok(event) => {
                        debug!("Parsed event: type={:?}, data={:?}", event.event_type, 
                                 if event.data.len() < 100 { &event.data } else { &event.data[..100] });
                        events.push(event);
                    }
                    Err(e) => {
                        error!("Failed to parse event block: {}", e);
                    }
                }
            }
        }
        
        debug!("Returning {} events, buffer remaining: {} chars", events.len(), self.buffer.len());
        Ok(events)
    }

    /// Parse a single event block
    fn parse_event_block(&self, block: &str) -> LLMResult<SSEEvent> {
        let mut event_type = None;
        let mut data_lines = Vec::new();
        let mut id = None;
        let mut retry = None;

        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') {
                continue; // Skip empty lines and comments
            }

            if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = if colon_pos + 1 < line.len() {
                    &line[colon_pos + 1..].trim_start()
                } else {
                    ""
                };

                match field {
                    "event" => event_type = Some(value.to_string()),
                    "data" => data_lines.push(value.to_string()),
                    "id" => id = Some(value.to_string()),
                    "retry" => {
                        retry = value.parse().ok();
                    }
                    _ => {} // Ignore unknown fields
                }
            } else {
                // Lines without colons are treated as data
                data_lines.push(line.to_string());
            }
        }

        Ok(SSEEvent {
            event_type,
            data: data_lines.join("\n"),
            id,
            retry,
        })
    }

    /// Check if there's remaining data in the buffer
    pub fn has_remaining_data(&self) -> bool {
        !self.buffer.trim().is_empty()
    }

    /// Get remaining data and clear buffer
    pub fn flush_remaining(&mut self) -> Option<String> {
        if self.buffer.trim().is_empty() {
            None
        } else {
            let remaining = self.buffer.clone();
            self.buffer.clear();
            Some(remaining)
        }
    }
}

/// Convert a reqwest Response into an SSE event stream
pub fn response_to_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = LLMResult<SSEEvent>> + Send + Unpin {
    let byte_stream = response.bytes_stream();
    let mut parser = SSEParser::new();

    Box::pin(byte_stream.map(move |chunk_result| {
        match chunk_result {
            Ok(chunk) => {
                match parser.parse_chunk(&chunk) {
                    Ok(events) => {
                        Ok(events)
                    }
                    Err(e) => {
                        Err(e)
                    }
                }
            }
            Err(e) => {
                Err(LLMError::Network(e.to_string()))
            }
        }
    }).filter_map(|result| async move {
        match result {
            Ok(events) => {
                if events.is_empty() {
                    debug!("Skipping empty event list");
                    None
                } else {
                    Some(Ok(events))
                }
            }
            Err(e) => Some(Err(e)),
        }
    }).flat_map(|events_result| {
        futures::stream::iter(match events_result {
            Ok(events) => events.into_iter().map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
        })
    }))
}

/// Anthropic-specific SSE parsing
pub mod anthropic {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type")]
    pub enum AnthropicStreamEvent {
        #[serde(rename = "ping")]
        Ping,
        
        #[serde(rename = "message_start")]
        MessageStart { message: AnthropicMessageStart },
        
        #[serde(rename = "content_block_start")]
        ContentBlockStart { 
            index: u32,
            content_block: AnthropicContentBlock,
        },
        
        #[serde(rename = "content_block_delta")]
        ContentBlockDelta {
            index: u32,
            delta: AnthropicDelta,
        },
        
        #[serde(rename = "content_block_stop")]
        ContentBlockStop { index: u32 },
        
        #[serde(rename = "message_delta")]
        MessageDelta { delta: AnthropicMessageDelta },
        
        #[serde(rename = "message_stop")]
        MessageStop,
        
        #[serde(rename = "error")]
        Error { error: AnthropicStreamError },
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicMessageStart {
        pub id: String,
        pub model: String,
        pub role: String,
        pub usage: AnthropicUsage,
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicContentBlock {
        #[serde(rename = "type")]
        pub block_type: String,
        pub text: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicDelta {
        #[serde(rename = "type")]
        pub delta_type: String,
        pub text: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicMessageDelta {
        pub stop_reason: Option<String>,
        pub usage: Option<AnthropicUsage>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicUsage {
        pub input_tokens: Option<u32>,
        pub output_tokens: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AnthropicStreamError {
        pub error_type: String,
        pub message: String,
    }

    /// Convert Anthropic SSE event to our StreamingChunk
    pub fn anthropic_event_to_chunk(
        event: &SSEEvent,
        request_id: &str,
        model: &str,
    ) -> LLMResult<Option<StreamingChunk>> {

        
        // Skip non-data events
        if event.data.trim().is_empty() || event.data.trim() == "[DONE]" {
            debug!("Skipping empty or DONE event");
            return Ok(None);
        }

        let stream_event: AnthropicStreamEvent = serde_json::from_str(&event.data)
            .map_err(|e| {
                error!("Failed to parse Anthropic JSON: {}", e);
                debug!("Raw data was: {}", event.data);
                LLMError::Parse(format!("Failed to parse Anthropic stream event: {}", e))
            })?;
            


        match stream_event {
            AnthropicStreamEvent::Ping => {
                debug!("Received ping event, ignoring");
                Ok(None) // Ignore ping events
            }
            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    Ok(Some(StreamingChunk {
                        id: request_id.to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.to_string(),
                        choices: vec![StreamingChoice {
                            index: 0,
                            delta: ChatMessage {
                                role: MessageRole::Assistant,
                                content: text,
                                name: None,
                                function_call: None,
                            },
                            finish_reason: None,
                        }],
                        provider: LLMProviderType::Anthropic,
                    }))
                } else {
                    debug!("Content delta with no text");
                    Ok(None)
                }
            }
            AnthropicStreamEvent::MessageDelta { delta } => {
                if let Some(stop_reason) = delta.stop_reason {
                    Ok(Some(StreamingChunk {
                        id: request_id.to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.to_string(),
                        choices: vec![StreamingChoice {
                            index: 0,
                            delta: ChatMessage {
                                role: MessageRole::Assistant,
                                content: String::new(),
                                name: None,
                                function_call: None,
                            },
                            finish_reason: Some(stop_reason),
                        }],
                        provider: LLMProviderType::Anthropic,
                    }))
                } else {
                    debug!("Message delta with no stop reason");
                    Ok(None)
                }
            }
            AnthropicStreamEvent::Error { error } => {
                Err(LLMError::Provider(format!("Anthropic stream error: {}", error.message)))
            }
            _ => {
                Ok(None) // Ignore other event types for now
            }
        }
    }
}

/// OpenAI-specific SSE parsing
pub mod openai {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct OpenAIStreamChunk {
        pub id: String,
        pub object: String,
        pub created: u64,
        pub model: String,
        pub choices: Vec<OpenAIStreamChoice>,
    }

    #[derive(Debug, Deserialize)]
    pub struct OpenAIStreamChoice {
        pub index: u32,
        pub delta: OpenAIDelta,
        pub finish_reason: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct OpenAIDelta {
        pub role: Option<String>,
        pub content: Option<String>,
    }

    /// Convert OpenAI SSE event to our StreamingChunk
    pub fn openai_event_to_chunk(event: &SSEEvent) -> LLMResult<Option<StreamingChunk>> {
        // Skip non-data events
        if event.data.trim().is_empty() || event.data.trim() == "[DONE]" {
            return Ok(None);
        }

        let chunk: OpenAIStreamChunk = serde_json::from_str(&event.data)
            .map_err(|e| LLMError::Parse(format!("Failed to parse OpenAI stream chunk: {}", e)))?;

        if let Some(choice) = chunk.choices.first() {
            let content = choice.delta.content.clone().unwrap_or_default();
            let role = choice.delta.role.clone().unwrap_or_else(|| "assistant".to_string());
            
            let message_role = match role.as_str() {
                "user" => MessageRole::User,
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::Assistant,
            };

            Ok(Some(StreamingChunk {
                id: chunk.id,
                object: chunk.object,
                created: chunk.created,
                model: chunk.model,
                choices: vec![StreamingChoice {
                    index: choice.index,
                    delta: ChatMessage {
                        role: message_role,
                        content,
                        name: None,
                        function_call: None,
                    },
                    finish_reason: choice.finish_reason.clone(),
                }],
                provider: LLMProviderType::OpenAI,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Google-specific SSE parsing
pub mod google {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct GoogleStreamChunk {
        pub candidates: Vec<GoogleCandidate>,
        #[serde(rename = "usageMetadata")]
        pub usage_metadata: Option<GoogleUsage>,
    }

    #[derive(Debug, Deserialize)]
    pub struct GoogleCandidate {
        pub content: GoogleContent,
        #[serde(rename = "finishReason")]
        pub finish_reason: Option<String>,
        pub index: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct GoogleContent {
        pub parts: Vec<GooglePart>,
        pub role: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct GooglePart {
        pub text: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct GoogleUsage {
        #[serde(rename = "promptTokenCount")]
        pub prompt_token_count: Option<u32>,
        #[serde(rename = "candidatesTokenCount")]
        pub candidates_token_count: Option<u32>,
        #[serde(rename = "totalTokenCount")]
        pub total_token_count: Option<u32>,
    }

    /// Convert Google SSE event to our StreamingChunk
    pub fn google_event_to_chunk(
        event: &SSEEvent,
        request_id: &str,
        model: &str,
    ) -> LLMResult<Option<StreamingChunk>> {
        // Skip non-data events
        if event.data.trim().is_empty() || event.data.trim() == "[DONE]" {
            return Ok(None);
        }

        let chunk: GoogleStreamChunk = serde_json::from_str(&event.data)
            .map_err(|e| LLMError::Parse(format!("Failed to parse Google stream chunk: {}", e)))?;

        if let Some(candidate) = chunk.candidates.first() {
            let content = candidate.content.parts
                .iter()
                .filter_map(|part| part.text.as_ref())
                .cloned()
                .collect::<Vec<String>>()
                .join("");

            if !content.is_empty() || candidate.finish_reason.is_some() {
                Ok(Some(StreamingChunk {
                    id: request_id.to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: model.to_string(),
                    choices: vec![StreamingChoice {
                        index: candidate.index.unwrap_or(0),
                        delta: ChatMessage {
                            role: MessageRole::Assistant,
                            content,
                            name: None,
                            function_call: None,
                        },
                        finish_reason: candidate.finish_reason.clone(),
                    }],
                    provider: LLMProviderType::Google,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_parser_basic() {
        let mut parser = SSEParser::new();
        
        let chunk = b"event: message\ndata: hello world\n\n";
        let events = parser.parse_chunk(chunk).unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, Some("message".to_string()));
        assert_eq!(events[0].data, "hello world");
    }

    #[test]
    fn test_sse_parser_multiple_events() {
        let mut parser = SSEParser::new();
        
        let chunk = b"data: first\n\ndata: second\n\n";
        let events = parser.parse_chunk(chunk).unwrap();
        
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn test_sse_parser_incomplete_event() {
        let mut parser = SSEParser::new();
        
        // First chunk with incomplete event
        let chunk1 = b"data: incomplete";
        let events1 = parser.parse_chunk(chunk1).unwrap();
        assert_eq!(events1.len(), 0);
        
        // Second chunk completes the event
        let chunk2 = b"\n\n";
        let events2 = parser.parse_chunk(chunk2).unwrap();
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "incomplete");
    }

    #[test]
    fn test_anthropic_content_delta_parsing() {
        let event = SSEEvent {
            event_type: None,
            data: r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#.to_string(),
            id: None,
            retry: None,
        };
        
        let chunk = anthropic::anthropic_event_to_chunk(&event, "test-id", "claude-3-sonnet").unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        assert_eq!(chunk.choices[0].delta.content, "Hello");
        assert_eq!(chunk.provider, LLMProviderType::Anthropic);
    }

    #[test]
    fn test_openai_delta_parsing() {
        let event = SSEEvent {
            event_type: None,
            data: r#"{"id":"test","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#.to_string(),
            id: None,
            retry: None,
        };
        
        let chunk = openai::openai_event_to_chunk(&event).unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        assert_eq!(chunk.choices[0].delta.content, "Hello");
        assert_eq!(chunk.provider, LLMProviderType::OpenAI);
    }
}