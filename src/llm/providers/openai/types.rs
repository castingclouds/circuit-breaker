//! OpenAI provider-specific types and structures
//! This module contains all the request/response types specific to OpenAI's API

use serde::{Deserialize, Serialize};
use crate::llm::{ChatMessage, TokenUsage};

/// OpenAI API request structure for chat completions
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

/// OpenAI chat message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Response format specification for structured outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String, // "text" or "json_object"
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String, // "function"
    pub function: Function,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool choice specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto(String), // "auto", "none"
    Specific { 
        #[serde(rename = "type")]
        choice_type: String,
        function: FunctionChoice 
    },
}

/// Specific function choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChoice {
    pub name: String,
}

/// Tool call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String, // "function"
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string
}

/// OpenAI API response structure
#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// OpenAI choice in response
#[derive(Debug, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// OpenAI usage statistics
#[derive(Debug, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

/// Prompt tokens breakdown
#[derive(Debug, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// Completion tokens breakdown
#[derive(Debug, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

/// OpenAI streaming response chunk
#[derive(Debug, Deserialize)]
pub struct OpenAIStreamingChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIStreamingChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// OpenAI streaming choice
#[derive(Debug, Deserialize)]
pub struct OpenAIStreamingChoice {
    pub index: u32,
    pub delta: ChatMessage,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// OpenAI error response
#[derive(Debug, Deserialize)]
pub struct OpenAIError {
    pub error: OpenAIErrorDetails,
}

/// OpenAI error details
#[derive(Debug, Deserialize)]
pub struct OpenAIErrorDetails {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// OpenAI model information
#[derive(Debug, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

/// OpenAI models list response
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIModel>,
}

impl From<&ChatMessage> for OpenAIChatMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: match msg.role {
                crate::llm::MessageRole::System => "system".to_string(),
                crate::llm::MessageRole::User => "user".to_string(),
                crate::llm::MessageRole::Assistant => "assistant".to_string(),
                crate::llm::MessageRole::Function => "function".to_string(),
            },
            content: msg.content.clone(),
            name: msg.name.clone(),
            tool_calls: None, // TODO: Implement tool calls conversion
            tool_call_id: None,
        }
    }
}

impl From<OpenAIUsage> for TokenUsage {
    fn from(usage: OpenAIUsage) -> Self {
        Self {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            estimated_cost: 0.0, // Will be calculated by cost calculator
        }
    }
}