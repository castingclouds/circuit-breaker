//! Ollama-specific request and response types
//!
//! This module contains type definitions that match Ollama's API format.
//! Ollama uses a different API structure compared to OpenAI-compatible APIs.

use serde::{Deserialize, Serialize};

/// Ollama chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaRequest {
    /// Model name (e.g., "llama2", "mistral", "codellama")
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<OllamaChatMessage>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Format to return a response in (e.g., "json")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Additional model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    /// System message to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Template to use for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Context parameter for maintaining conversation state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,
    /// Keep alive parameter for model lifecycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
    /// Tools available for function calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OllamaTool>>,
}

/// Ollama chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaChatMessage {
    /// Role of the message sender
    pub role: String,
    /// Content of the message
    pub content: String,
    /// Optional images for multimodal models (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OllamaToolCall>>,
}

/// Ollama model options/parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaOptions {
    /// Temperature for randomness (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Repeat penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f64>,
    /// Seed for reproducible outputs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    /// Number of context tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<i32>,
    /// Number of threads to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_thread: Option<i32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

/// Ollama chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaResponse {
    /// Model used for the response
    pub model: String,
    /// Timestamp when response was created
    pub created_at: String,
    /// The response message
    pub message: OllamaChatMessage,
    /// Whether this is the final response
    pub done: bool,
    /// Total duration in nanoseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    /// Load duration in nanoseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    /// Prompt evaluation count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    /// Prompt evaluation duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    /// Evaluation count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    /// Evaluation duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
    /// Context data for conversation continuity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,
}

/// Ollama streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaStreamingChunk {
    /// Model used for the response
    pub model: String,
    /// Timestamp when response was created
    pub created_at: String,
    /// The response message (partial for streaming)
    pub message: OllamaChatMessage,
    /// Whether this is the final chunk
    pub done: bool,
    /// Metrics (only present in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Ollama model information from /api/tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    /// Model name
    pub name: String,
    /// Model size in bytes
    pub size: u64,
    /// Model digest/hash
    pub digest: String,
    /// Model details
    pub details: OllamaModelDetails,
    /// When the model was modified
    pub modified_at: String,
}

/// Detailed model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    /// Format (e.g., "gguf")
    pub format: String,
    /// Model family (e.g., "llama")
    pub family: String,
    /// Parameter size (e.g., "7B")
    pub parameter_size: String,
    /// Quantization level (e.g., "Q4_0")
    pub quantization_level: String,
    /// Parent model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_model: Option<String>,
}

/// Response from /api/tags endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    /// List of available models
    pub models: Vec<OllamaModelInfo>,
}

/// Ollama error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaError {
    /// Error message
    pub error: String,
}

/// Generate request for /api/generate endpoint (non-chat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaGenerateRequest {
    /// Model name
    pub model: String,
    /// Prompt text
    pub prompt: String,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Additional model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    /// System message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Template to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Context for conversation continuity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,
    /// Keep alive setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
    /// Images for multimodal models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

/// Generate response from /api/generate endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaGenerateResponse {
    /// Model used
    pub model: String,
    /// Timestamp
    pub created_at: String,
    /// Generated response text
    pub response: String,
    /// Whether generation is complete
    pub done: bool,
    /// Context for next request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<i32>>,
    /// Performance metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaHealthResponse {
    /// Status message
    pub status: String,
}

/// Ollama embeddings request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingsRequest {
    /// Model name for embeddings
    pub model: String,
    /// Text to generate embeddings for
    pub prompt: String,
    /// Additional model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    /// Keep alive parameter for model lifecycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/// Ollama embeddings response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingsResponse {
    /// The embedding vector
    pub embedding: Vec<f64>,
}

/// Batch embeddings request for multiple texts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaBatchEmbeddingsRequest {
    /// Model name for embeddings
    pub model: String,
    /// Texts to generate embeddings for
    pub input: Vec<String>,
    /// Additional model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    /// Keep alive parameter for model lifecycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/// Batch embeddings response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaBatchEmbeddingsResponse {
    /// Array of embedding vectors
    pub embeddings: Vec<Vec<f64>>,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaTool {
    #[serde(rename = "type")]
    pub tool_type: String, // "function"
    pub function: OllamaFunction,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaToolCall {
    pub function: OllamaFunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaFunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

impl From<&crate::llm::ChatMessage> for OllamaChatMessage {
    fn from(msg: &crate::llm::ChatMessage) -> Self {
        let tool_calls = msg.function_call.as_ref().map(|fc| {
            vec![OllamaToolCall {
                function: OllamaFunctionCall {
                    name: fc.name.clone(),
                    arguments: serde_json::from_str(&fc.arguments)
                        .unwrap_or(serde_json::Value::Null),
                },
            }]
        });

        Self {
            role: match msg.role {
                crate::llm::MessageRole::System => "system".to_string(),
                crate::llm::MessageRole::User => "user".to_string(),
                crate::llm::MessageRole::Assistant => "assistant".to_string(),
                crate::llm::MessageRole::Function => "assistant".to_string(), // Map function to assistant
            },
            content: msg.content.clone(),
            images: None, // TODO: Add support for multimodal when needed
            tool_calls,
        }
    }
}

impl From<OllamaChatMessage> for crate::llm::ChatMessage {
    fn from(msg: OllamaChatMessage) -> Self {
        let function_call = msg
            .tool_calls
            .as_ref()
            .and_then(|tool_calls| tool_calls.first())
            .map(|tool_call| crate::llm::FunctionCall {
                name: tool_call.function.name.clone(),
                arguments: tool_call.function.arguments.to_string(),
            });

        Self {
            role: match msg.role.as_str() {
                "system" => crate::llm::MessageRole::System,
                "user" => crate::llm::MessageRole::User,
                "assistant" => crate::llm::MessageRole::Assistant,
                _ => crate::llm::MessageRole::Assistant, // Default to assistant
            },
            content: msg.content,
            name: None,
            function_call,
        }
    }
}
