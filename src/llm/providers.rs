// LLM Provider implementations for different services
// This module contains the actual provider clients that interface with external LLM APIs

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use super::{
    LLMError, LLMRequest, LLMResponse, LLMResult, StreamingChunk, 
    Choice, StreamingChoice, ChatMessage, MessageRole, TokenUsage, LLMProviderType, RoutingInfo, RoutingStrategy
};
use chrono;

#[async_trait]
pub trait LLMProviderClient: Send + Sync {
    /// Send a chat completion request
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse>;

    /// Send a streaming chat completion request
    async fn chat_completion_stream(
        &self,
        request: &LLMRequest,
        api_key: &str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>>;

    /// Get the provider type
    fn provider_type(&self) -> LLMProviderType;

    /// Health check for the provider
    async fn health_check(&self, api_key: &str) -> LLMResult<bool>;
}

/// Anthropic Claude provider
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
        }
    }

    fn build_headers(&self, api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(api_key).unwrap());
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers
    }

    fn convert_request(&self, request: &LLMRequest) -> serde_json::Value {
        json!({
            "model": request.model,
            "messages": request.messages.iter().map(|msg| {
                json!({
                    "role": match msg.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user", 
                        MessageRole::Assistant => "assistant",
                        MessageRole::Function => "assistant"
                    },
                    "content": msg.content
                })
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens.unwrap_or(1000),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": false
        })
    }
}

#[async_trait]
impl LLMProviderClient for AnthropicProvider {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        let headers = self.build_headers(api_key);
        let payload = self.convert_request(request);

        let response = self
            .client
            .post(&format!("{}/v1/messages", self.base_url))
            .headers(headers)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return match status.as_u16() {
                401 => Err(LLMError::AuthenticationFailed(error_text)),
                429 => Err(LLMError::RateLimitExceeded(error_text)),
                400 => Err(LLMError::InvalidRequest(error_text)),
                _ => Err(LLMError::Internal(format!(
                    "HTTP {}: {}",
                    status, error_text
                ))),
            };
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        let model_name = anthropic_response.model.clone();
        Ok(LLMResponse {
            id: anthropic_response.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: anthropic_response.model,
            choices: anthropic_response
                .content
                .iter()
                .enumerate()
                .map(|(index, content_block)| Choice {
                    index: index as u32,
                    message: ChatMessage {
                        role: MessageRole::Assistant,
                        content: content_block.text.clone(),
                        name: None,
                        function_call: None,
                    },
                    finish_reason: Some(anthropic_response.stop_reason.clone().unwrap_or_else(|| {
                        if anthropic_response.stop_sequence.is_some() {
                            "stop".to_string()
                        } else {
                            "length".to_string()
                        }
                    })),
                })
                .collect(),
            usage: TokenUsage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
                estimated_cost: calculate_anthropic_cost(
                    anthropic_response.usage.input_tokens,
                    anthropic_response.usage.output_tokens,
                    &model_name,
                ),
            },
            provider: LLMProviderType::Anthropic,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::Anthropic,
                routing_strategy: RoutingStrategy::ModelSpecific("anthropic".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
            },
        })
    }

    async fn chat_completion_stream(
        &self,
        request: &LLMRequest,
        api_key: &str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        let headers = self.build_headers(api_key);
        let mut payload = self.convert_request(request);
        payload["stream"] = json!(true);

        let response = self
            .client
            .post(&format!("{}/v1/messages", self.base_url))
            .headers(headers)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return match status.as_u16() {
                401 => Err(LLMError::AuthenticationFailed(error_text)),
                429 => Err(LLMError::RateLimitExceeded(error_text)),
                400 => Err(LLMError::InvalidRequest(error_text)),
                _ => Err(LLMError::Internal(format!(
                    "HTTP {}: {}",
                    status, error_text
                ))),
            };
        }

        // Convert response stream to our streaming format
        let stream = response.bytes_stream();
        let mapped_stream = stream.map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    
                    // Look for Anthropic streaming events
                    for line in chunk_str.lines() {
                        if line.starts_with("data: ") {
                            let data_content = line.strip_prefix("data: ").unwrap_or("").trim();
                            
                            // Skip heartbeat and done messages
                            if data_content.is_empty() || data_content == "[DONE]" {
                                continue;
                            }
                            
                            if let Ok(chunk_data) = serde_json::from_str::<serde_json::Value>(data_content) {
                                // Handle Anthropic streaming events
                                if let Some(event_type) = chunk_data.get("type").and_then(|t| t.as_str()) {
                                    if event_type == "content_block_delta" {
                                        if let Some(text) = chunk_data
                                            .get("delta")
                                            .and_then(|d| d.get("text"))
                                            .and_then(|t| t.as_str()) {
                                            
                                            let streaming_chunk = StreamingChunk {
                                                id: "anthropic-stream".to_string(),
                                                object: "chat.completion.chunk".to_string(),
                                                created: chrono::Utc::now().timestamp() as u64,
                                                model: "claude-sonnet-4-20250514".to_string(),
                                                choices: vec![StreamingChoice {
                                                    index: 0,
                                                    delta: ChatMessage {
                                                        role: MessageRole::Assistant,
                                                        content: text.to_string(),
                                                        name: None,
                                                        function_call: None,
                                                    },
                                                    finish_reason: None,
                                                }],
                                                provider: LLMProviderType::Anthropic,
                                            };
                                            return Ok(streaming_chunk);
                                        }
                                    } else if event_type == "message_stop" {
                                        let streaming_chunk = StreamingChunk {
                                            id: "anthropic-stream".to_string(),
                                            object: "chat.completion.chunk".to_string(),
                                            created: chrono::Utc::now().timestamp() as u64,
                                            model: "claude-sonnet-4-20250514".to_string(),
                                            choices: vec![StreamingChoice {
                                                index: 0,
                                                delta: ChatMessage {
                                                    role: MessageRole::Assistant,
                                                    content: String::new(),
                                                    name: None,
                                                    function_call: None,
                                                },
                                                finish_reason: Some("stop".to_string()),
                                            }],
                                            provider: LLMProviderType::Anthropic,
                                        };
                                        return Ok(streaming_chunk);
                                    }
                                }
                            }
                        }
                    }
                    
                    // If no valid content found, return empty chunk
                    Ok(StreamingChunk {
                        id: "anthropic-empty".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: "claude-sonnet-4-20250514".to_string(),
                        choices: vec![],
                        provider: LLMProviderType::Anthropic,
                    })
                },
                Err(e) => Err(LLMError::Network(e.to_string())),
            }
        });

        // Filter out empty chunks
        let filtered_stream = mapped_stream.filter_map(|result| async move {
            match result {
                Ok(chunk) if chunk.choices.is_empty() => None,
                other => Some(other),
            }
        });

        Ok(Box::new(Box::pin(filtered_stream)))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Anthropic
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let headers = self.build_headers(api_key);
        
        let response = self
            .client
            .get(&format!("{}/v1/messages", self.base_url))
            .headers(headers)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().as_u16() != 401),
            Err(_) => Ok(false),
        }
    }
}

// Anthropic API response structures
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<AnthropicContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Calculate cost for Anthropic models
fn calculate_anthropic_cost(input_tokens: u32, output_tokens: u32, model: &str) -> f64 {
    let (input_cost_per_token, output_cost_per_token) = match model {
        "claude-3-haiku-20240307" => (0.00000025, 0.00000125),
        "claude-3-sonnet-20240229" => (0.000003, 0.000015),
        "claude-sonnet-4-20250514" => (0.000003, 0.000015),
        _ => (0.000003, 0.000015), // Default to Sonnet pricing
    };

    (input_tokens as f64 * input_cost_per_token) + (output_tokens as f64 * output_cost_per_token)
}

/// Factory function to create LLM provider clients
pub fn create_provider_client(
    provider_type: &LLMProviderType,
    base_url: Option<String>,
) -> Box<dyn LLMProviderClient> {
    match provider_type {
        LLMProviderType::Anthropic => Box::new(AnthropicProvider::new(base_url)),
        _ => {
            // For other providers, default to Anthropic for now
            Box::new(AnthropicProvider::new(base_url))
        }
    }
}