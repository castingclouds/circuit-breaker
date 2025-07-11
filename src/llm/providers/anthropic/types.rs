//! Anthropic provider-specific types and structures
//! This module contains all the request/response types specific to Anthropic's API

use crate::llm::{ChatMessage, MessageRole, TokenUsage};
use serde::{Deserialize, Serialize};

/// Anthropic API request structure for chat completions
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
}

/// Anthropic tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Anthropic message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

/// Anthropic API response structure
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<AnthropicContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic content block
#[derive(Debug, Deserialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

/// Anthropic usage statistics
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Anthropic streaming response chunk
#[derive(Debug, Deserialize)]
pub struct AnthropicStreamingChunk {
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub index: Option<u32>,
    pub delta: Option<AnthropicDelta>,
    pub usage: Option<AnthropicUsage>,
}

/// Anthropic streaming delta
#[derive(Debug, Deserialize)]
pub struct AnthropicDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
    pub stop_reason: Option<String>,
}

/// Anthropic error response
#[derive(Debug, Deserialize)]
pub struct AnthropicError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: AnthropicErrorDetails,
}

/// Anthropic error details
#[derive(Debug, Deserialize)]
pub struct AnthropicErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl From<&ChatMessage> for AnthropicMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: match msg.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Function => "user".to_string(), // Anthropic doesn't have function role
            },
            content: msg.content.clone(),
        }
    }
}

impl From<AnthropicUsage> for TokenUsage {
    fn from(usage: AnthropicUsage) -> Self {
        Self {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.input_tokens + usage.output_tokens,
            estimated_cost: 0.0, // Will be calculated by cost calculator
        }
    }
}

impl AnthropicResponse {
    /// Convert to our internal ChatMessage format
    pub fn to_chat_message(&self) -> ChatMessage {
        let mut content_parts = Vec::new();
        let mut function_call = None;

        for block in &self.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(ref text) = block.text {
                        content_parts.push(text.clone());
                    }
                }
                "tool_use" => {
                    // Convert Anthropic tool use to our function_call format
                    if let (Some(ref name), Some(ref input)) = (&block.name, &block.input) {
                        function_call = Some(crate::llm::FunctionCall {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        });
                    }
                }
                _ => {
                    // Handle other block types if needed
                }
            }
        }

        let content = content_parts.join("\n");

        ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call,
        }
    }
}
