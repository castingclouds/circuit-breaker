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
    async fn chat_completion_stream<'a>(
        &'a self,
        request: &'a LLMRequest,
        api_key: &'a str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin + 'a>>;

    /// Get the provider type
    fn provider_type(&self) -> LLMProviderType;

    /// Health check for the provider
    async fn health_check(&self, api_key: &str) -> LLMResult<bool>;
}

/// OpenAI provider
pub struct OpenAIProvider {
    client: Client,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }

    fn build_headers(&self, api_key: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap());
        headers
    }

    fn convert_request(&self, request: &LLMRequest) -> serde_json::Value {
        let max_tokens_value = request.max_tokens.unwrap_or(1000);
        
        // Some newer OpenAI models (like o4-mini series) require max_completion_tokens instead of max_tokens
        let (max_tokens_key, max_tokens_val) = if request.model.starts_with("o4-") {
            ("max_completion_tokens", max_tokens_value)
        } else {
            ("max_tokens", max_tokens_value)
        };
        
        // Some models like o4-mini only support default temperature (1.0)
        let temperature = if request.model.starts_with("o4-") {
            1.0 // o4 models require default temperature
        } else {
            request.temperature.unwrap_or(0.7)
        };
        
        json!({
            "model": request.model,
            "messages": request.messages.iter().map(|msg| {
                json!({
                    "role": match msg.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user", 
                        MessageRole::Assistant => "assistant",
                        MessageRole::Function => "function"
                    },
                    "content": msg.content
                })
            }).collect::<Vec<_>>(),
            max_tokens_key: max_tokens_val,
            "temperature": temperature,
            "stream": false
        })
    }
}

#[async_trait]
impl LLMProviderClient for OpenAIProvider {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        let headers = self.build_headers(api_key);
        let payload = self.convert_request(request);

        let request_url = format!("{}/chat/completions", self.base_url);
        eprintln!("🔍 OpenAI API Request:");
        eprintln!("   URL: {}", request_url);
        eprintln!("   Model: {}", request.model);
        eprintln!("   Base URL: {}", self.base_url);
        eprintln!("   Headers: Authorization: Bearer {}...", &api_key[..8]);
        eprintln!("   Payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response = self
            .client
            .post(&request_url)
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

            eprintln!("❌ OpenAI API Error: {} - {}", status, error_text);
            eprintln!("   Full response body: {}", error_text);
            eprintln!("   Request URL was: {}", request_url);
            eprintln!("   Model requested: {}", request.model);

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

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        Ok(LLMResponse {
            id: openai_response.id,
            object: openai_response.object,
            created: openai_response.created,
            model: openai_response.model.clone(),
            choices: openai_response
                .choices
                .into_iter()
                .map(|choice| Choice {
                    index: choice.index,
                    message: choice.message,
                    finish_reason: choice.finish_reason,
                })
                .collect(),
            usage: TokenUsage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
                estimated_cost: calculate_openai_cost(
                    openai_response.usage.prompt_tokens,
                    openai_response.usage.completion_tokens,
                    &openai_response.model,
                ),
            },
            provider: LLMProviderType::OpenAI,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::OpenAI,
                routing_strategy: RoutingStrategy::ModelSpecific("openai".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
            },
        })
    }

    async fn chat_completion_stream<'a>(
        &'a self,
        request: &'a LLMRequest,
        api_key: &'a str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin + 'a>> {
        let headers = self.build_headers(api_key);
        let mut payload = self.convert_request(request);
        payload["stream"] = json!(true);

        let request_url = format!("{}/chat/completions", self.base_url);
        eprintln!("🔍 OpenAI Streaming Request:");
        eprintln!("   URL: {}", request_url);
        eprintln!("   Model: {}", request.model);
        eprintln!("   Stream: true");

        let response = self
            .client
            .post(&request_url)
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

            eprintln!("❌ Google API Error: {} - {}", status, error_text);
            eprintln!("   Full response body: {}", error_text);
            eprintln!("   Request URL was: {}", request_url);
            eprintln!("   Model requested: {}", request.model);
            eprintln!("   Base URL: {}", self.base_url);

            return match status.as_u16() {
                401 => Err(LLMError::AuthenticationFailed(error_text)),
                429 => Err(LLMError::RateLimitExceeded(error_text)),
                400 => Err(LLMError::InvalidRequest(error_text)),
                404 => Err(LLMError::Internal(format!(
                    "Model not found - HTTP 404: {}",
                    error_text
                ))),
                _ => Err(LLMError::Internal(format!(
                    "HTTP {}: {}",
                    status, error_text
                ))),
            };
        }

        // Convert response stream to our streaming format
        let stream = response.bytes_stream();
        let model_clone = request.model.clone();
        let mapped_stream = stream.map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    
                    // Look for OpenAI streaming events
                    for line in chunk_str.lines() {
                        if line.starts_with("data: ") {
                            let data_content = line.strip_prefix("data: ").unwrap_or("").trim();
                            
                            // Skip heartbeat and done messages
                            if data_content.is_empty() || data_content == "[DONE]" {
                                continue;
                            }
                            
                            if let Ok(chunk_data) = serde_json::from_str::<OpenAIStreamingChunk>(data_content) {
                                let streaming_chunk = StreamingChunk {
                                    id: chunk_data.id,
                                    object: chunk_data.object,
                                    created: chunk_data.created,
                                    model: chunk_data.model,
                                    choices: chunk_data.choices.into_iter().map(|choice| StreamingChoice {
                                        index: choice.index,
                                        delta: choice.delta,
                                        finish_reason: choice.finish_reason,
                                    }).collect(),
                                    provider: LLMProviderType::OpenAI,
                                };
                                return Ok(streaming_chunk);
                            }
                        }
                    }
                    
                    // If no valid content found, return empty chunk
                    Ok(StreamingChunk {
                        id: "openai-empty".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model_clone.clone(),
                        choices: vec![],
                        provider: LLMProviderType::OpenAI,
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
        LLMProviderType::OpenAI
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let headers = self.build_headers(api_key);
        
        let response = self
            .client
            .get(&format!("{}/models", self.base_url))
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

/// Google Gemini provider
pub struct GoogleProvider {
    client: Client,
    base_url: String,
}

impl GoogleProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
        }
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    fn convert_request(&self, request: &LLMRequest) -> serde_json::Value {
        let contents = request.messages.iter().map(|msg| {
            json!({
                "parts": [{"text": msg.content}],
                "role": match msg.role {
                    MessageRole::System => "user", // Gemini doesn't have system role
                    MessageRole::User => "user",
                    MessageRole::Assistant => "model",
                    MessageRole::Function => "user"
                }
            })
        }).collect::<Vec<_>>();

        json!({
            "contents": contents,
            "generationConfig": {
                "temperature": request.temperature.unwrap_or(0.7),
                "maxOutputTokens": request.max_tokens.unwrap_or(1000),
                "topP": request.top_p.unwrap_or(0.95)
            }
        })
    }
}

#[async_trait]
impl LLMProviderClient for GoogleProvider {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        let headers = self.build_headers();
        let payload = self.convert_request(request);

        let request_url = format!("{}/models/{}:generateContent?key={}", 
                         self.base_url, request.model, api_key);

        // Mask API key for security
        let masked_key = if api_key.len() > 8 {
            format!("{}...{}", &api_key[..4], &api_key[api_key.len()-4..])
        } else {
            "***".to_string()
        };
        let debug_url = format!("{}/models/{}:generateContent?key={}", 
                               self.base_url, request.model, masked_key);

        eprintln!("🔍 Google API Request:");
        eprintln!("   URL: {}", debug_url);
        eprintln!("   Model: {}", request.model);
        eprintln!("   Base URL: {}", self.base_url);
        eprintln!("   Full URL with model: {}/models/{}:generateContent", self.base_url, request.model);
        eprintln!("   API Key: {}...", &api_key[..8]);
        eprintln!("   Content-Type: application/json");
        eprintln!("   Payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());

        let response = self
            .client
            .post(&request_url)
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

            eprintln!("❌ OpenAI Streaming Error: {} - {}", status, error_text);
            eprintln!("   Model: {}", request.model);
            eprintln!("   URL: {}", request_url);

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

        let google_response: GoogleResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        let content = google_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(LLMResponse {
            id: format!("google-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: request.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content,
                    name: None,
                    function_call: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: TokenUsage {
                prompt_tokens: google_response.usage_metadata.prompt_token_count,
                completion_tokens: google_response.usage_metadata.candidates_token_count,
                total_tokens: google_response.usage_metadata.total_token_count,
                estimated_cost: calculate_google_cost(
                    google_response.usage_metadata.prompt_token_count,
                    google_response.usage_metadata.candidates_token_count,
                    &request.model,
                ),
            },
            provider: LLMProviderType::Google,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::Google,
                routing_strategy: RoutingStrategy::ModelSpecific("google".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
            },
        })
    }

    async fn chat_completion_stream<'a>(
        &'a self,
        request: &'a LLMRequest,
        api_key: &'a str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin + 'a>> {
        let headers = self.build_headers();
        let payload = self.convert_request(request);

        let request_url = format!("{}/models/{}:streamGenerateContent?key={}", 
                         self.base_url, request.model, api_key);

        let response = self
            .client
            .post(&request_url)
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
        let model_clone = request.model.clone();
        let mapped_stream = stream.map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    
                    // Google streaming format is different - looking for JSON objects
                    for line in chunk_str.lines() {
                        if let Ok(chunk_data) = serde_json::from_str::<GoogleStreamingChunk>(line) {
                            if let Some(candidate) = chunk_data.candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    let streaming_chunk = StreamingChunk {
                                        id: format!("google-stream-{}", chrono::Utc::now().timestamp()),
                                        object: "chat.completion.chunk".to_string(),
                                        created: chrono::Utc::now().timestamp() as u64,
                                        model: model_clone.clone(),
                                        choices: vec![StreamingChoice {
                                            index: 0,
                                            delta: ChatMessage {
                                                role: MessageRole::Assistant,
                                                content: part.text.clone(),
                                                name: None,
                                                function_call: None,
                                            },
                                            finish_reason: candidate.finish_reason.clone(),
                                        }],
                                        provider: LLMProviderType::Google,
                                    };
                                    return Ok(streaming_chunk);
                                }
                            }
                        }
                    }
                    
                    // If no valid content found, return empty chunk
                    Ok(StreamingChunk {
                        id: "google-empty".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model_clone.clone(),
                        choices: vec![],
                        provider: LLMProviderType::Google,
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
        LLMProviderType::Google
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let headers = self.build_headers();
        
        let request_url = format!("{}/models?key={}", self.base_url, api_key);
        let response = self
            .client
            .get(&request_url)
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

    async fn chat_completion_stream<'a>(
        &'a self,
        request: &'a LLMRequest,
        api_key: &'a str,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin + 'a>> {
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

// OpenAI API response structures
#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIStreamingChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIStreamingChoice>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIStreamingChoice {
    pub index: u32,
    pub delta: ChatMessage,
    pub finish_reason: Option<String>,
}

// Google API response structures
#[derive(Debug, Deserialize)]
pub struct GoogleResponse {
    pub candidates: Vec<GoogleCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: GoogleUsage,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCandidate {
    pub content: GoogleContent,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleContent {
    pub parts: Vec<GooglePart>,
}

#[derive(Debug, Deserialize)]
pub struct GooglePart {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUsage {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct GoogleStreamingChunk {
    pub candidates: Vec<GoogleCandidate>,
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

/// Calculate cost for OpenAI models
fn calculate_openai_cost(input_tokens: u32, output_tokens: u32, model: &str) -> f64 {
    let (input_cost_per_token, output_cost_per_token) = match model {
        "gpt-4" => (0.00003, 0.00006),
        "gpt-4-turbo" => (0.00001, 0.00003),
        "gpt-3.5-turbo" => (0.000001, 0.000002),
        "gpt-4o" => (0.000005, 0.000015),
        "gpt-4o-mini" => (0.00000015, 0.0000006),
        _ => (0.00003, 0.00006), // Default to GPT-4 pricing
    };

    (input_tokens as f64 * input_cost_per_token) + (output_tokens as f64 * output_cost_per_token)
}

/// Calculate cost for Google models
fn calculate_google_cost(input_tokens: u32, output_tokens: u32, model: &str) -> f64 {
    let (input_cost_per_token, output_cost_per_token) = match model {
        "gemini-pro" => (0.0000005, 0.0000015),
        "gemini-pro-vision" => (0.00000025, 0.00000125),
        "gemini-1.5-pro" => (0.0000035, 0.0000105),
        "gemini-1.5-flash" => (0.000000075, 0.0000003),
        _ => (0.0000005, 0.0000015), // Default to Gemini Pro pricing
    };

    (input_tokens as f64 * input_cost_per_token) + (output_tokens as f64 * output_cost_per_token)
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
        LLMProviderType::OpenAI => Box::new(OpenAIProvider::new(base_url)),
        LLMProviderType::Anthropic => Box::new(AnthropicProvider::new(base_url)),
        LLMProviderType::Google => Box::new(GoogleProvider::new(base_url)),
        _ => {
            // For other providers, default to OpenAI for now
            Box::new(OpenAIProvider::new(base_url))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_openai_provider() {
        let provider = create_provider_client(&LLMProviderType::OpenAI, None);
        assert_eq!(provider.provider_type(), LLMProviderType::OpenAI);
    }

    #[test]
    fn test_create_anthropic_provider() {
        let provider = create_provider_client(&LLMProviderType::Anthropic, None);
        assert_eq!(provider.provider_type(), LLMProviderType::Anthropic);
    }

    #[test]
    fn test_create_google_provider() {
        let provider = create_provider_client(&LLMProviderType::Google, None);
        assert_eq!(provider.provider_type(), LLMProviderType::Google);
    }

    #[test]
    fn test_cost_calculations() {
        // Test OpenAI cost calculation
        let openai_cost = calculate_openai_cost(100, 50, "gpt-4");
        assert!(openai_cost > 0.0);

        // Test Google cost calculation
        let google_cost = calculate_google_cost(100, 50, "gemini-pro");
        assert!(google_cost > 0.0);

        // Test Anthropic cost calculation
        let anthropic_cost = calculate_anthropic_cost(100, 50, "claude-3-sonnet-20240229");
        assert!(anthropic_cost > 0.0);

        // Verify that different models have different costs
        let gpt4_cost = calculate_openai_cost(100, 50, "gpt-4");
        let gpt35_cost = calculate_openai_cost(100, 50, "gpt-3.5-turbo");
        assert!(gpt4_cost > gpt35_cost);
    }
}