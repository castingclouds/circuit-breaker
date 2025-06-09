// OpenAI-compatible API types and schemas
// This module defines the request/response types that match OpenAI's API specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAI Chat Completion Request
/// Matches the OpenAI API specification for /v1/chat/completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// ID of the model to use
    pub model: String,
    
    /// A list of messages comprising the conversation so far
    pub messages: Vec<ChatMessage>,
    
    /// What sampling temperature to use, between 0 and 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// The maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Whether to return a stream of partial results
    #[serde(default)]
    pub stream: bool,
    
    /// Number of chat completion choices to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    
    /// Up to 4 sequences where the API will stop generating further tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    
    /// An alternative to sampling with temperature, called nucleus sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    
    /// A unique identifier representing your end-user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    
    /// Circuit Breaker smart routing configuration (optional extension)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    
    /// Additional provider-specific parameters
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// OpenAI Chat Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author
    pub role: ChatRole,
    
    /// The contents of the message
    pub content: String,
    
    /// The name of the author of this message (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Tool calls that the model wants to make (for function calling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    
    /// Tool call ID (for tool responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Chat message roles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
    Function, // Deprecated but still supported
}

/// Tool call structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI Chat Completion Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// A unique identifier for the chat completion
    pub id: String,
    
    /// The object type, which is always "chat.completion"
    pub object: String,
    
    /// The Unix timestamp (in seconds) when the chat completion was created
    pub created: u64,
    
    /// The model used for the chat completion
    pub model: String,
    
    /// A list of chat completion choices
    pub choices: Vec<ChatCompletionChoice>,
    
    /// Usage statistics for the completion request
    pub usage: Usage,
    
    /// The system fingerprint of the model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// Chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    /// The index of the choice in the list of choices
    pub index: u32,
    
    /// The chat completion message
    pub message: ChatMessage,
    
    /// The reason the model stopped generating tokens
    pub finish_reason: Option<String>,
    
    /// Log probability information for the choice tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    
    /// Number of tokens in the generated completion
    pub completion_tokens: u32,
    
    /// Total number of tokens used in the request (prompt + completion)
    pub total_tokens: u32,
}

/// OpenAI Streaming Chat Completion Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponse {
    /// A unique identifier for the chat completion
    pub id: String,
    
    /// The object type, which is always "chat.completion.chunk"
    pub object: String,
    
    /// The Unix timestamp (in seconds) when the chat completion was created
    pub created: u64,
    
    /// The model used for the chat completion
    pub model: String,
    
    /// A list of chat completion choices
    pub choices: Vec<ChatCompletionStreamChoice>,
    
    /// The system fingerprint of the model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// Streaming chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionStreamChoice {
    /// The index of the choice in the list of choices
    pub index: u32,
    
    /// The delta (partial message) for this chunk
    pub delta: ChatMessageDelta,
    
    /// The reason the model stopped generating tokens
    pub finish_reason: Option<String>,
    
    /// Log probability information for the choice tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Delta message for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDelta {
    /// The role of the message author (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatRole>,
    
    /// The content delta (partial content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    /// Tool calls delta (for function calling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Tool call delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}

/// Function call delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// OpenAI Models List Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// The object type, which is always "list"
    pub object: String,
    
    /// List of model objects
    pub data: Vec<Model>,
}

/// OpenAI Model Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier
    pub id: String,
    
    /// The object type, which is always "model"
    pub object: String,
    
    /// The Unix timestamp (in seconds) when the model was created
    pub created: u64,
    
    /// The organization that owns the model
    pub owned_by: String,
    
    /// Additional model metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// OpenAI Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

/// Error detail structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Error message
    pub message: String,
    
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    
    /// Parameter that caused the error (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    
    /// Error code (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

// ============================================================================
// Smart Routing Extensions for Circuit Breaker
// These maintain 100% OpenAI API compatibility while adding intelligent routing
// ============================================================================

/// Circuit Breaker smart routing configuration
/// Optional extension that can be included in OpenAI requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Routing strategy preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_strategy: Option<String>,
    
    /// Maximum cost per 1K tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_per_1k_tokens: Option<f64>,
    
    /// Maximum latency in milliseconds  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<u64>,
    
    /// Task type for optimal model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    
    /// Fallback models if primary selection fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_models: Option<Vec<String>>,
    
    /// Preferred providers (in priority order)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_providers: Option<Vec<String>>,
}

/// Smart routing strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SmartRoutingStrategy {
    #[serde(rename = "cost_optimized")]
    CostOptimized,
    #[serde(rename = "performance_first")]
    PerformanceFirst,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "reliability_first")]
    ReliabilityFirst,
    #[serde(rename = "task_specific")]
    TaskSpecific,
}

/// Task types for smart model selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    #[serde(rename = "general_chat")]
    GeneralChat,
    #[serde(rename = "coding")]
    Coding,
    #[serde(rename = "analysis")]
    Analysis,
    #[serde(rename = "creative")]
    Creative,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "summarization")]
    Summarization,
}

/// Virtual model mapping for smart routing
#[derive(Debug, Clone)]
pub struct VirtualModel {
    pub id: String,
    pub description: String,
    pub strategy: SmartRoutingStrategy,
    pub task_type: Option<TaskType>,
    pub max_cost: Option<f64>,
}

impl VirtualModel {
    pub fn new(id: &str, description: &str, strategy: SmartRoutingStrategy) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            strategy,
            task_type: None,
            max_cost: None,
        }
    }
    
    pub fn with_task_type(mut self, task_type: TaskType) -> Self {
        self.task_type = Some(task_type);
        self
    }
    
    pub fn with_max_cost(mut self, max_cost: f64) -> Self {
        self.max_cost = Some(max_cost);
        self
    }
}

/// Helper function to get default virtual models
pub fn get_virtual_models() -> Vec<VirtualModel> {
    vec![
        VirtualModel::new("auto", "Automatically select best available model", SmartRoutingStrategy::Balanced),
        VirtualModel::new("cb:smart-chat", "Smart chat model selection", SmartRoutingStrategy::Balanced)
            .with_task_type(TaskType::GeneralChat),
        VirtualModel::new("cb:cost-optimal", "Most cost-effective model", SmartRoutingStrategy::CostOptimized),
        VirtualModel::new("cb:fastest", "Fastest responding model", SmartRoutingStrategy::PerformanceFirst),
        VirtualModel::new("cb:coding", "Best model for code generation", SmartRoutingStrategy::TaskSpecific)
            .with_task_type(TaskType::Coding),
        VirtualModel::new("cb:analysis", "Best model for data analysis", SmartRoutingStrategy::TaskSpecific)
            .with_task_type(TaskType::Analysis),
        VirtualModel::new("cb:creative", "Best model for creative tasks", SmartRoutingStrategy::TaskSpecific)
            .with_task_type(TaskType::Creative),
    ]
}

/// Check if a model name is a virtual model
pub fn is_virtual_model(model_name: &str) -> bool {
    model_name == "auto" || model_name.starts_with("cb:")
}

/// Server-Sent Events wrapper for streaming responses
#[derive(Debug, Clone)]
pub struct SSEData {
    pub data: String,
    pub event: Option<String>,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

impl SSEData {
    pub fn new(data: String) -> Self {
        Self {
            data,
            event: None,
            id: None,
            retry: None,
        }
    }
    
    pub fn with_event(mut self, event: String) -> Self {
        self.event = Some(event);
        self
    }
    
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }
    
    pub fn with_retry(mut self, retry: u64) -> Self {
        self.retry = Some(retry);
        self
    }
    
    /// Format as Server-Sent Events format
    pub fn to_sse_string(&self) -> String {
        let mut result = String::new();
        
        if let Some(event) = &self.event {
            result.push_str(&format!("event: {}\n", event));
        }
        
        if let Some(id) = &self.id {
            result.push_str(&format!("id: {}\n", id));
        }
        
        if let Some(retry) = &self.retry {
            result.push_str(&format!("retry: {}\n", retry));
        }
        
        // Handle multi-line data
        for line in self.data.lines() {
            result.push_str(&format!("data: {}\n", line));
        }
        
        result.push('\n');
        result
    }
}

/// Helper function to create an error response
pub fn create_error_response(message: String, error_type: String, param: Option<String>, code: Option<String>) -> ErrorResponse {
    ErrorResponse {
        error: ErrorDetail {
            message,
            error_type,
            param,
            code,
        },
    }
}

/// Helper function to generate a completion ID
pub fn generate_completion_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..27].to_string())
}

/// Helper function to get current Unix timestamp
pub fn current_timestamp() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

/// Convert internal ChatMessage to OpenAI format
impl From<crate::llm::ChatMessage> for ChatMessage {
    fn from(msg: crate::llm::ChatMessage) -> Self {
        Self {
            role: match msg.role {
                crate::llm::MessageRole::System => ChatRole::System,
                crate::llm::MessageRole::User => ChatRole::User,
                crate::llm::MessageRole::Assistant => ChatRole::Assistant,
                crate::llm::MessageRole::Function => ChatRole::Function,
            },
            content: msg.content,
            name: msg.name,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// Convert OpenAI ChatMessage to internal format
impl From<ChatMessage> for crate::llm::ChatMessage {
    fn from(msg: ChatMessage) -> Self {
        Self {
            role: match msg.role {
                ChatRole::System => crate::llm::MessageRole::System,
                ChatRole::User => crate::llm::MessageRole::User,
                ChatRole::Assistant => crate::llm::MessageRole::Assistant,
                ChatRole::Tool | ChatRole::Function => crate::llm::MessageRole::Function,
            },
            content: msg.content,
            name: msg.name,
            function_call: None,
        }
    }
}

/// Convert ChatCompletionRequest to internal LLMRequest
impl From<ChatCompletionRequest> for crate::llm::LLMRequest {
    fn from(req: ChatCompletionRequest) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            model: req.model,
            messages: req.messages.into_iter().map(|m| m.into()).collect(),
            temperature: req.temperature.map(|t| t as f64),
            max_tokens: req.max_tokens,
            top_p: req.top_p.map(|p| p as f64),
            frequency_penalty: req.frequency_penalty.map(|p| p as f64),
            presence_penalty: req.presence_penalty.map(|p| p as f64),
            stop: req.stop,
            stream: Some(req.stream),
            functions: None,
            function_call: None,
            user: req.user,
            metadata: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sse_data_formatting() {
        let sse = SSEData::new("Hello, world!".to_string())
            .with_event("message".to_string())
            .with_id("123".to_string());
            
        let formatted = sse.to_sse_string();
        assert!(formatted.contains("event: message\n"));
        assert!(formatted.contains("id: 123\n"));
        assert!(formatted.contains("data: Hello, world!\n"));
    }
    
    #[test]
    fn test_completion_id_generation() {
        let id = generate_completion_id();
        assert!(id.starts_with("chatcmpl-"));
        assert_eq!(id.len(), 36); // "chatcmpl-" + 27 chars
    }
    
    #[test]
    fn test_chat_message_conversion() {
        let internal_msg = crate::llm::ChatMessage {
            role: crate::llm::MessageRole::User,
            content: "Hello".to_string(),
            name: None,
            function_call: None,
        };
        
        let openai_msg: ChatMessage = internal_msg.into();
        assert_eq!(openai_msg.content, "Hello");
        assert!(matches!(openai_msg.role, ChatRole::User));
        
        let back_to_internal: crate::llm::ChatMessage = openai_msg.into();
        assert_eq!(back_to_internal.content, "Hello");
        assert!(matches!(back_to_internal.role, crate::llm::MessageRole::User));
    }
}