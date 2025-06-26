//! LLM module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for interacting with Large Language Models
//! through the Circuit Breaker router using OpenAI-compatible API calls.

use crate::{Client, Result};
use serde::{Deserialize, Serialize};

/// Common LLM models used across providers
pub mod common_models {
    // Virtual Models (Circuit Breaker Smart Routing)
    pub const SMART_FAST: &str = "cb:fastest";
    pub const SMART_CHEAP: &str = "cb:cost-optimal";
    pub const SMART_BALANCED: &str = "cb:smart-chat";
    pub const SMART_CREATIVE: &str = "cb:creative";
    pub const SMART_CODING: &str = "cb:coding";
    pub const SMART_ANALYSIS: &str = "cb:analysis";

    // Direct Provider Models
    pub const GPT_O4_MINI: &str = "o4-mini-2025-04-16";
    pub const CLAUDE_4_SONNET: &str = "claude-sonnet-4-20250514";
    pub const GEMINI_PRO: &str = "gemini-2.5-pro";
}

/// Client for LLM operations through Circuit Breaker router
#[derive(Debug, Clone)]
pub struct LLMClient {
    client: Client,
}

impl LLMClient {
    /// Create a new LLM client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Make a smart completion request with Circuit Breaker routing
    pub async fn smart_completion(
        &self,
        request: SmartCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        self.chat_completion(request.into()).await
    }

    /// Make a chat completion request through the Circuit Breaker router
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        self.client
            .rest(reqwest::Method::POST, "/v1/chat/completions", Some(request))
            .await
    }

    /// Create a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl futures::Stream<Item = Result<ChatCompletionChunk>>> {
        let mut streaming_request = request;
        streaming_request.stream = Some(true);

        // Use smart REST routing instead of direct URL construction
        let rest_endpoint = self.client.get_endpoint_url("rest");
        let url = format!("{}/v1/chat/completions", rest_endpoint);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/event-stream"),
        );

        // Add API key if available
        if let Some(api_key) = self.client.api_key() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(
                    |e| crate::Error::Configuration {
                        message: format!("Invalid API key format: {}", e),
                    },
                )?,
            );
        }

        let response = self
            .client
            .http_client()
            .post(&url)
            .headers(headers)
            .json(&streaming_request)
            .timeout(std::time::Duration::from_millis(self.client.timeout_ms()))
            .send()
            .await
            .map_err(|e| crate::Error::Network {
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::Error::Network {
                message: format!("HTTP {} - {}", status, error_text),
            });
        }

        let stream = response.bytes_stream();
        let sse_stream = self.parse_sse_stream(stream);

        Ok(sse_stream)
    }

    /// Parse Server-Sent Events stream into chat completion chunks
    fn parse_sse_stream(
        &self,
        stream: impl futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static,
    ) -> impl futures::Stream<Item = Result<ChatCompletionChunk>> {
        use futures::StreamExt;

        stream
            .map(|chunk_result| {
                chunk_result.map_err(|e| crate::Error::Network {
                    message: format!("Stream error: {}", e),
                })
            })
            .scan(String::new(), |buffer, chunk_result| {
                let buffer = std::mem::take(buffer);
                async move {
                    let mut buffer = buffer;
                    match chunk_result {
                        Ok(chunk) => {
                            let chunk_str = String::from_utf8_lossy(&chunk);
                            buffer.push_str(&chunk_str);

                            let mut events = Vec::new();
                            let lines = buffer.lines().collect::<Vec<_>>();

                            // Find complete events (ending with double newline)
                            let mut i = 0;
                            while i < lines.len() {
                                if lines[i].starts_with("data: ") {
                                    let data = &lines[i][6..]; // Remove "data: " prefix

                                    if data == "[DONE]" {
                                        events.push(Err(crate::Error::Stream {
                                            message: "Stream completed".to_string(),
                                        }));
                                        break;
                                    }

                                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                                        Ok(chunk) => events.push(Ok(chunk)),
                                        Err(e) => events.push(Err(crate::Error::Parse {
                                            message: format!("Failed to parse chunk: {}", e),
                                        })),
                                    }
                                }
                                i += 1;
                            }

                            // Keep incomplete data in buffer
                            if let Some(last_complete) =
                                lines.iter().rposition(|line| line.is_empty())
                            {
                                buffer = lines[last_complete + 1..].join("\n");
                            }

                            Some((buffer, futures::stream::iter(events)))
                        }
                        Err(e) => Some((buffer, futures::stream::iter(vec![Err(e)]))),
                    }
                }
            })
            .map(|item| item.1)
            .flatten()
    }

    /// Get available models from the Circuit Breaker router
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let models_response: ModelsResponse = self
            .client
            .rest(reqwest::Method::GET, "/v1/models", None::<()>)
            .await?;

        Ok(models_response.data)
    }

    /// Simple chat method with just model and message
    pub async fn chat(&self, model: &str, message: &str) -> Result<String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: message.to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: Some(false),
            ..Default::default()
        };

        let response = self.chat_completion(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(crate::Error::Parse {
                message: "No response choices available".to_string(),
            })
        }
    }

    /// Chat with system prompt
    pub async fn chat_with_system(
        &self,
        model: &str,
        system: &str,
        message: &str,
    ) -> Result<String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: system.to_string(),
                    name: None,
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: message.to_string(),
                    name: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: Some(false),
            ..Default::default()
        };

        let response = self.chat_completion(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(crate::Error::Parse {
                message: "No response choices available".to_string(),
            })
        }
    }
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<ChatFunction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerOptions>,
}

impl Default for ChatCompletionRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
            functions: None,
            function_call: None,
            circuit_breaker: None,
        }
    }
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<TokenUsage>,
}

/// Chat completion chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoiceDelta>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Chat role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Function,
}

/// Chat choice in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Chat choice delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoiceDelta {
    pub index: u32,
    pub delta: ChatMessageDelta,
    pub finish_reason: Option<String>,
}

/// Chat message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Function definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    pub owned_by: String,
}

/// Models response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

/// OpenAI error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIErrorResponse {
    pub error: OpenAIError,
}

/// OpenAI error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    pub code: Option<String>,
}

/// Smart completion request with Circuit Breaker routing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerOptions>,
}

/// Circuit Breaker specific routing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_strategy: Option<RoutingStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_per_1k_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<TaskType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_constraint: Option<BudgetConstraint>,
}

/// Routing strategies for smart model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    CostOptimized,
    PerformanceFirst,
    LoadBalanced,
    FailoverChain,
    ModelSpecific,
}

/// Task types for optimized routing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    #[serde(rename = "general_chat")]
    General,
    #[serde(rename = "coding")]
    CodeGeneration,
    #[serde(rename = "analysis")]
    DataAnalysis,
    #[serde(rename = "creative")]
    CreativeWriting,
    #[serde(rename = "reasoning")]
    Reasoning,
}

/// Budget constraint options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConstraint {
    pub daily_limit: Option<f64>,
    pub monthly_limit: Option<f64>,
    pub per_request_limit: Option<f64>,
}

impl From<SmartCompletionRequest> for ChatCompletionRequest {
    fn from(smart_request: SmartCompletionRequest) -> Self {
        ChatCompletionRequest {
            model: smart_request.model,
            messages: smart_request.messages,
            temperature: smart_request.temperature,
            max_tokens: smart_request.max_tokens,
            stream: smart_request.stream,
            circuit_breaker: smart_request.circuit_breaker,
            ..Default::default()
        }
    }
}

/// Builder for creating chat requests
pub struct ChatBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    stop: Option<Vec<String>>,
    user: Option<String>,
    functions: Option<Vec<ChatFunction>>,
    circuit_breaker: Option<CircuitBreakerOptions>,
}

impl ChatBuilder {
    /// Create a new chat builder
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
            functions: None,
            circuit_breaker: None,
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::System,
            content: prompt.into(),
            name: None,
        });
        self
    }

    /// Add user message
    pub fn add_user_message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: message.into(),
            name: None,
        });
        self
    }

    /// Add assistant message
    pub fn add_assistant_message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: message.into(),
            name: None,
        });
        self
    }

    /// Set temperature
    pub fn set_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn set_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set streaming
    pub fn set_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set top_p
    pub fn set_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set frequency penalty
    pub fn set_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set presence penalty
    pub fn set_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences
    pub fn set_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set user identifier
    pub fn set_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Add function for function calling
    pub fn add_function(mut self, function: ChatFunction) -> Self {
        if self.functions.is_none() {
            self.functions = Some(Vec::new());
        }
        self.functions.as_mut().unwrap().push(function);
        self
    }

    /// Set Circuit Breaker routing options
    pub fn set_circuit_breaker(mut self, options: CircuitBreakerOptions) -> Self {
        self.circuit_breaker = Some(options);
        self
    }

    /// Set routing strategy for smart routing
    pub fn set_routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        if self.circuit_breaker.is_none() {
            self.circuit_breaker = Some(CircuitBreakerOptions {
                routing_strategy: None,
                max_cost_per_1k_tokens: None,
                max_latency_ms: None,
                fallback_models: None,
                task_type: None,
                require_streaming: None,
                budget_constraint: None,
            });
        }
        self.circuit_breaker.as_mut().unwrap().routing_strategy = Some(strategy);
        self
    }

    /// Set maximum cost per 1k tokens
    pub fn set_max_cost_per_1k_tokens(mut self, max_cost: f64) -> Self {
        if self.circuit_breaker.is_none() {
            self.circuit_breaker = Some(CircuitBreakerOptions {
                routing_strategy: None,
                max_cost_per_1k_tokens: None,
                max_latency_ms: None,
                fallback_models: None,
                task_type: None,
                require_streaming: None,
                budget_constraint: None,
            });
        }
        self.circuit_breaker
            .as_mut()
            .unwrap()
            .max_cost_per_1k_tokens = Some(max_cost);
        self
    }

    /// Set task type for optimized routing
    pub fn set_task_type(mut self, task_type: TaskType) -> Self {
        if self.circuit_breaker.is_none() {
            self.circuit_breaker = Some(CircuitBreakerOptions {
                routing_strategy: None,
                max_cost_per_1k_tokens: None,
                max_latency_ms: None,
                fallback_models: None,
                task_type: None,
                require_streaming: None,
                budget_constraint: None,
            });
        }
        self.circuit_breaker.as_mut().unwrap().task_type = Some(task_type);
        self
    }

    /// Set fallback models
    pub fn set_fallback_models(mut self, models: Vec<String>) -> Self {
        if self.circuit_breaker.is_none() {
            self.circuit_breaker = Some(CircuitBreakerOptions {
                routing_strategy: None,
                max_cost_per_1k_tokens: None,
                max_latency_ms: None,
                fallback_models: None,
                task_type: None,
                require_streaming: None,
                budget_constraint: None,
            });
        }
        self.circuit_breaker.as_mut().unwrap().fallback_models = Some(models);
        self
    }

    /// Build the chat request
    pub fn build(self) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: self.model,
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
            top_p: self.top_p,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            stop: self.stop,
            user: self.user,
            functions: self.functions,
            function_call: None,
            circuit_breaker: self.circuit_breaker,
        }
    }

    /// Execute the chat request
    pub async fn execute(self, client: &LLMClient) -> Result<ChatCompletionResponse> {
        client.chat_completion(self.build()).await
    }

    /// Execute as streaming request
    pub async fn execute_stream(
        self,
        client: &LLMClient,
    ) -> Result<impl futures::Stream<Item = Result<ChatCompletionChunk>>> {
        client.chat_completion_stream(self.build()).await
    }
}

/// Convenience function to create a chat builder
pub fn create_chat(model: impl Into<String>) -> ChatBuilder {
    ChatBuilder::new(model)
}

/// Convenience function to create a smart chat builder with virtual model
pub fn create_smart_chat(virtual_model: impl Into<String>) -> ChatBuilder {
    ChatBuilder::new(virtual_model)
}

/// Create a cost-optimized chat request
pub fn create_cost_optimized_chat() -> ChatBuilder {
    ChatBuilder::new(common_models::SMART_CHEAP)
        .set_routing_strategy(RoutingStrategy::CostOptimized)
}

/// Create a performance-optimized chat request
pub fn create_fast_chat() -> ChatBuilder {
    ChatBuilder::new(common_models::SMART_FAST)
        .set_routing_strategy(RoutingStrategy::PerformanceFirst)
}

/// Create a balanced chat request
pub fn create_balanced_chat() -> ChatBuilder {
    ChatBuilder::new(common_models::SMART_BALANCED)
        .set_routing_strategy(RoutingStrategy::LoadBalanced)
}

/// Helper to create a simple chat request
pub fn chat_request(model: &str, message: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        stream: Some(false),
        ..Default::default()
    }
}

/// Helper to create a smart chat request with routing options
pub fn smart_chat_request(
    virtual_model: &str,
    message: &str,
    routing_strategy: RoutingStrategy,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: virtual_model.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        stream: Some(false),
        circuit_breaker: Some(CircuitBreakerOptions {
            routing_strategy: Some(routing_strategy),
            max_cost_per_1k_tokens: None,
            max_latency_ms: None,
            fallback_models: None,
            task_type: None,
            require_streaming: None,
            budget_constraint: None,
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_builder() {
        let request = create_chat("gpt-4")
            .set_system_prompt("You are a helpful assistant")
            .add_user_message("Hello, world!")
            .set_temperature(0.8)
            .set_max_tokens(150)
            .build();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.8));
        assert_eq!(request.max_tokens, Some(150));
    }

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: ChatRole::User,
            content: "Hello".to_string(),
            name: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_chat_completion_request_serialization() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Test".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_chat_completion_request_default() {
        let request = ChatCompletionRequest::default();
        assert!(request.model.is_empty());
        assert!(request.messages.is_empty());
        assert!(request.temperature.is_none());
        assert!(request.max_tokens.is_none());
        assert!(request.stream.is_none());
    }

    #[test]
    fn test_chat_builder_with_functions() {
        let function = ChatFunction {
            name: "get_weather".to_string(),
            description: Some("Get weather".to_string()),
            parameters: serde_json::json!({"type": "object"}),
        };

        let request = create_chat("gpt-4")
            .add_user_message("What's the weather?")
            .add_function(function)
            .set_user("test-user")
            .build();

        assert_eq!(request.messages.len(), 1);
        assert!(request.functions.is_some());
        assert_eq!(request.functions.as_ref().unwrap().len(), 1);
        assert_eq!(request.user, Some("test-user".to_string()));
    }

    #[test]
    fn test_chat_builder_advanced_parameters() {
        let request = create_chat("gpt-4")
            .add_user_message("Test")
            .set_top_p(0.9)
            .set_frequency_penalty(0.5)
            .set_presence_penalty(0.3)
            .set_stop(vec!["STOP".to_string(), "END".to_string()])
            .build();

        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.frequency_penalty, Some(0.5));
        assert_eq!(request.presence_penalty, Some(0.3));
        assert_eq!(
            request.stop,
            Some(vec!["STOP".to_string(), "END".to_string()])
        );
    }

    #[test]
    fn test_chat_role_serialization() {
        assert_eq!(
            serde_json::to_string(&ChatRole::System).unwrap(),
            "\"system\""
        );
        assert_eq!(serde_json::to_string(&ChatRole::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&ChatRole::Assistant).unwrap(),
            "\"assistant\""
        );
        assert_eq!(
            serde_json::to_string(&ChatRole::Function).unwrap(),
            "\"function\""
        );
    }

    #[test]
    fn test_chat_request_helper() {
        let request = chat_request("gpt-3.5-turbo", "Hello world");

        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, ChatRole::User);
        assert_eq!(request.messages[0].content, "Hello world");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.stream, Some(false));
    }

    #[test]
    fn test_openai_error_deserialization() {
        let error_json = r#"{
            "error": {
                "message": "Invalid request",
                "type": "invalid_request_error",
                "param": "model",
                "code": "model_not_found"
            }
        }"#;

        let error: OpenAIErrorResponse = serde_json::from_str(error_json).unwrap();
        assert_eq!(error.error.message, "Invalid request");
        assert_eq!(
            error.error.error_type,
            Some("invalid_request_error".to_string())
        );
        assert_eq!(error.error.param, Some("model".to_string()));
        assert_eq!(error.error.code, Some("model_not_found".to_string()));
    }

    #[test]
    fn test_model_constants() {
        // Virtual models
        assert_eq!(common_models::SMART_FAST, "cb:fastest");
        assert_eq!(common_models::SMART_CHEAP, "cb:cost-optimal");
        assert_eq!(common_models::SMART_BALANCED, "cb:smart-chat");
        assert_eq!(common_models::SMART_CREATIVE, "cb:creative");

        // Direct models
        assert_eq!(common_models::GPT_O4_MINI, "o4-mini-2025-04-16");
        assert_eq!(common_models::CLAUDE_4_SONNET, "claude-sonnet-4-20250514");
        assert_eq!(common_models::GEMINI_PRO, "gemini-2.5-pro");
    }

    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage {
            prompt_tokens: 50,
            completion_tokens: 25,
            total_tokens: 75,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"prompt_tokens\":50"));
        assert!(json.contains("\"completion_tokens\":25"));
        assert!(json.contains("\"total_tokens\":75"));
    }

    #[test]
    fn test_chat_function_serialization() {
        let function = ChatFunction {
            name: "test_function".to_string(),
            description: Some("Test function".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }),
        };

        let json = serde_json::to_string(&function).unwrap();
        assert!(json.contains("\"name\":\"test_function\""));
        assert!(json.contains("\"description\":\"Test function\""));
        assert!(json.contains("\"properties\""));
    }

    #[test]
    fn test_circuit_breaker_options_serialization() {
        let options = CircuitBreakerOptions {
            routing_strategy: Some(RoutingStrategy::CostOptimized),
            max_cost_per_1k_tokens: Some(0.01),
            task_type: Some(TaskType::CodeGeneration),
            fallback_models: Some(vec!["gpt-3.5-turbo".to_string()]),
            max_latency_ms: None,
            require_streaming: None,
            budget_constraint: None,
        };

        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"routing_strategy\":\"cost_optimized\""));
        assert!(json.contains("\"max_cost_per_1k_tokens\":0.01"));
        assert!(json.contains("\"task_type\":\"coding\""));
    }

    #[test]
    fn test_smart_completion_request() {
        let request = SmartCompletionRequest {
            model: common_models::SMART_CHEAP.to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Test message".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
            circuit_breaker: Some(CircuitBreakerOptions {
                routing_strategy: Some(RoutingStrategy::CostOptimized),
                max_cost_per_1k_tokens: Some(0.01),
                task_type: Some(TaskType::General),
                fallback_models: None,
                max_latency_ms: None,
                require_streaming: None,
                budget_constraint: None,
            }),
        };

        assert_eq!(request.model, "smart-cheap");
        assert!(request.circuit_breaker.is_some());
    }

    #[test]
    fn test_chat_builder_with_circuit_breaker() {
        let request = create_cost_optimized_chat()
            .add_user_message("Generate code for sorting")
            .set_task_type(TaskType::CodeGeneration)
            .set_max_cost_per_1k_tokens(0.02)
            .build();

        assert_eq!(request.model, "smart-cheap");
        assert!(request.circuit_breaker.is_some());
        let cb = request.circuit_breaker.unwrap();
        assert!(matches!(
            cb.routing_strategy,
            Some(RoutingStrategy::CostOptimized)
        ));
        assert!(matches!(cb.task_type, Some(TaskType::CodeGeneration)));
        assert_eq!(cb.max_cost_per_1k_tokens, Some(0.02));
    }

    #[test]
    fn test_smart_chat_helpers() {
        let cost_request = smart_chat_request(
            common_models::SMART_CHEAP,
            "Hello",
            RoutingStrategy::CostOptimized,
        );
        assert_eq!(cost_request.model, "smart-cheap");
        assert!(cost_request.circuit_breaker.is_some());

        let fast_builder = create_fast_chat();
        let fast_request = fast_builder.add_user_message("Quick question").build();
        assert_eq!(fast_request.model, "smart-fast");
    }
}
