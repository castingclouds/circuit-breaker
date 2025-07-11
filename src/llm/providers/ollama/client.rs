//! Ollama provider client implementation
//! This module contains the actual client that makes requests to Ollama's API

use async_trait::async_trait;
use futures::Stream;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use crate::llm::{
    Choice, EmbeddingData, EmbeddingsInput, EmbeddingsRequest, EmbeddingsResponse, EmbeddingsUsage,
    LLMError, LLMProviderType, LLMRequest, LLMResponse, LLMResult, RoutingInfo, RoutingStrategy,
    StreamingChunk, TokenUsage,
};

use crate::llm::traits::{LLMProviderClient, ModelInfo, ProviderConfigRequirements};

use super::config::{get_config_requirements, supports_function_calling, OllamaConfig};
use super::types::{
    OllamaChatMessage, OllamaEmbeddingsRequest, OllamaEmbeddingsResponse, OllamaError,
    OllamaModelInfo, OllamaModelsResponse, OllamaOptions, OllamaRequest, OllamaResponse,
    OllamaStreamingChunk,
};

/// Ollama provider client
pub struct OllamaClient {
    client: Client,
    config: OllamaConfig,
}

impl OllamaClient {
    /// Create a new Ollama client with configuration
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    /// Get available models with async support (preferred method)
    pub async fn get_available_models_async(&self) -> Vec<ModelInfo> {
        match self.fetch_available_models_from_openai_api().await {
            Ok(models) => {
                debug!("Fetched {} models from Ollama OpenAI API", models.len());
                models
            }
            Err(e) => {
                warn!(
                    "Failed to fetch models from Ollama OpenAI API, using configured default: {}",
                    e
                );
                // Return just the configured default model as a fallback
                vec![self.create_model_info_for_configured_default()]
            }
        }
    }

    /// Create a new Ollama client with default configuration
    pub fn with_base_url(base_url: String) -> Self {
        let mut config = OllamaConfig::default();
        config.base_url = base_url;
        Self::new(config)
    }

    /// Build HTTP headers for requests
    fn build_headers(&self, _api_key: &str) -> LLMResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add custom headers if any
        for (key, value) in &self.config.custom_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| LLMError::Internal(format!("Invalid header key: {}", e)))?;
            headers.insert(
                header_name,
                HeaderValue::from_str(value)
                    .map_err(|e| LLMError::Internal(format!("Invalid header value: {}", e)))?,
            );
        }

        Ok(headers)
    }

    /// Convert our internal request format to Ollama's format
    fn convert_request(&self, request: &LLMRequest) -> LLMResult<OllamaRequest> {
        let messages: Vec<OllamaChatMessage> =
            request.messages.iter().map(|msg| msg.into()).collect();

        // Convert parameters to Ollama options
        let mut options = OllamaOptions {
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop.clone(),
            ..Default::default()
        };

        // Map max_tokens to num_predict for Ollama
        if let Some(max_tokens) = request.max_tokens {
            options.num_predict = Some(max_tokens as i32);
        }

        // Handle system message - Ollama can accept it as a separate field
        let (system_message, filtered_messages) = extract_system_message(messages);

        // Convert functions for function calling
        let tools = request.functions.as_ref().map(|functions| {
            functions
                .iter()
                .map(|function| super::types::OllamaTool {
                    tool_type: "function".to_string(),
                    function: super::types::OllamaFunction {
                        name: function.name.clone(),
                        description: function.description.clone(),
                        parameters: function.parameters.clone(),
                    },
                })
                .collect()
        });

        let ollama_request = OllamaRequest {
            model: request.model.clone(),
            messages: filtered_messages,
            stream: request.stream,
            format: None, // Could be "json" for structured output
            options: Some(options),
            system: system_message,
            template: None,
            context: None, // TODO: Implement conversation context
            keep_alive: Some(self.config.keep_alive.clone()),
            tools,
        };

        Ok(ollama_request)
    }

    /// Convert Ollama response to our internal format
    fn convert_response(
        &self,
        ollama_response: OllamaResponse,
        request_id: &str,
        start_time: Instant,
    ) -> LLMResult<LLMResponse> {
        let choice = Choice {
            index: 0,
            message: ollama_response.message.into(),
            finish_reason: if ollama_response.done {
                Some("stop".to_string())
            } else {
                None
            },
        };

        // Calculate token usage from Ollama metrics
        let usage = TokenUsage {
            prompt_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
            completion_tokens: ollama_response.eval_count.unwrap_or(0),
            total_tokens: ollama_response.prompt_eval_count.unwrap_or(0)
                + ollama_response.eval_count.unwrap_or(0),
            estimated_cost: 0.0, // Local inference is free
        };

        let routing_info = RoutingInfo {
            selected_provider: LLMProviderType::Ollama,
            routing_strategy: RoutingStrategy::ModelSpecific(ollama_response.model.clone()),
            latency_ms: start_time.elapsed().as_millis() as u64,
            retry_count: 0,
            fallback_used: false,
            provider_used: LLMProviderType::Ollama,
            total_latency_ms: start_time.elapsed().as_millis() as u64,
            provider_latency_ms: ollama_response
                .total_duration
                .map(|d| d / 1_000_000)
                .unwrap_or(0), // Convert nanoseconds to milliseconds
        };

        Ok(LLMResponse {
            id: request_id.to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: ollama_response.model,
            choices: vec![choice],
            usage,
            provider: LLMProviderType::Ollama,
            routing_info,
        })
    }

    /// Fetch available models from Ollama's OpenAI-compatible API
    async fn fetch_available_models_from_openai_api(&self) -> LLMResult<Vec<ModelInfo>> {
        let headers = self.build_headers("")?; // Ollama typically doesn't require auth
        let url = format!("{}/v1/models", self.config.base_url);

        debug!("Fetching available models from OpenAI API: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!(
                "Model fetch failed with status {}: {}",
                status, error_text
            )));
        }

        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(format!("Failed to parse models response: {}", e)))?;

        // Parse OpenAI-compatible models response
        let models = models_response
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| LLMError::Parse("Invalid models response format".to_string()))?;

        let model_infos = models
            .iter()
            .filter_map(|model| {
                let id = model.get("id")?.as_str()?;
                Some(self.create_model_info_from_id(id))
            })
            .collect();

        Ok(model_infos)
    }

    /// Fallback: Fetch available models from Ollama's native API
    async fn fetch_available_models(&self) -> LLMResult<Vec<ModelInfo>> {
        let headers = self.build_headers("")?; // Ollama typically doesn't require auth
        let url = format!("{}/api/tags", self.config.base_url);

        debug!("Fetching available models from native API: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!(
                "Model fetch failed with status {}: {}",
                status, error_text
            )));
        }

        let models_response: OllamaModelsResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(format!("Failed to parse models response: {}", e)))?;

        // Convert Ollama model info to our ModelInfo format
        let model_infos = models_response
            .models
            .into_iter()
            .map(|model| convert_ollama_model_to_info(model))
            .collect();

        Ok(model_infos)
    }
}

#[async_trait]
impl LLMProviderClient for OllamaClient {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        debug!(
            "Starting Ollama chat completion for model: {}",
            request.model
        );

        let ollama_request = self.convert_request(request)?;
        let url = format!("{}/api/chat", self.config.base_url);
        let headers = self.build_headers(api_key)?;

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as Ollama error
            if let Ok(ollama_error) = serde_json::from_str::<OllamaError>(&error_text) {
                return Err(LLMError::Provider(ollama_error.error));
            }

            return Err(LLMError::Provider(format!(
                "Request failed with status {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(format!("Failed to parse response: {}", e)))?;

        self.convert_response(ollama_response, &request_id, start_time)
    }

    async fn chat_completion_stream(
        &self,
        request: LLMRequest,
        api_key: String,
    ) -> LLMResult<Box<dyn Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        debug!(
            "Starting Ollama streaming chat completion for model: {}",
            request.model
        );

        let mut ollama_request = self.convert_request(&request)?;
        ollama_request.stream = Some(true);

        let url = format!("{}/api/chat", self.config.base_url);
        let headers = self.build_headers(&api_key)?;

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Stream request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!(
                "Stream request failed with status {}: {}",
                status, error_text
            )));
        }

        let stream = OllamaStreamAdapter::new(response.bytes_stream(), request.model.clone());
        Ok(Box::new(stream))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Ollama
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let url = format!("{}/api/tags", self.config.base_url);
        let headers = self.build_headers(api_key)?;

        debug!("Performing Ollama health check at: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .timeout(Duration::from_secs(10)) // Shorter timeout for health checks
            .send()
            .await
            .map_err(|e| {
                warn!("Ollama health check failed: {}", e);
                LLMError::Network(format!("Health check failed: {}", e))
            })?;

        let is_healthy = response.status().is_success();
        debug!("Ollama health check result: {}", is_healthy);

        Ok(is_healthy)
    }

    fn get_available_models(&self) -> Vec<ModelInfo> {
        // For synchronous context, return just the configured default model
        // This avoids blocking issues while still providing the user's configured model
        vec![self.create_model_info_for_configured_default()]
    }

    fn supports_model(&self, model: &str) -> bool {
        // For dynamic models, be permissive and let Ollama decide
        // This allows any model that might be available on the Ollama instance
        true
    }

    fn get_config_requirements(&self) -> ProviderConfigRequirements {
        get_config_requirements()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn embeddings(
        &self,
        request: &EmbeddingsRequest,
        api_key: &str,
    ) -> LLMResult<EmbeddingsResponse> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        debug!("Starting Ollama embeddings for model: {}", request.model);

        match &request.input {
            EmbeddingsInput::Text(text) => {
                let embedding = self
                    .get_single_embedding(&request.model, text, api_key)
                    .await?;

                let data = vec![EmbeddingData {
                    index: 0,
                    embedding,
                    object: "embedding".to_string(),
                }];

                let usage = EmbeddingsUsage {
                    prompt_tokens: estimate_tokens(text),
                    total_tokens: estimate_tokens(text),
                    estimated_cost: 0.0, // Local inference is free
                };

                let routing_info = RoutingInfo {
                    selected_provider: LLMProviderType::Ollama,
                    routing_strategy: RoutingStrategy::ModelSpecific(request.model.clone()),
                    latency_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    fallback_used: false,
                    provider_used: LLMProviderType::Ollama,
                    total_latency_ms: start_time.elapsed().as_millis() as u64,
                    provider_latency_ms: start_time.elapsed().as_millis() as u64,
                };

                Ok(EmbeddingsResponse {
                    id: request_id,
                    object: "list".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: request.model.clone(),
                    data,
                    usage,
                    provider: LLMProviderType::Ollama,
                    routing_info,
                })
            }
            EmbeddingsInput::TextArray(texts) => {
                let mut embeddings_data = Vec::new();
                let mut total_tokens = 0;

                for (index, text) in texts.iter().enumerate() {
                    let embedding = self
                        .get_single_embedding(&request.model, text, api_key)
                        .await?;

                    embeddings_data.push(EmbeddingData {
                        index: index as u32,
                        embedding,
                        object: "embedding".to_string(),
                    });

                    total_tokens += estimate_tokens(text);
                }

                let usage = EmbeddingsUsage {
                    prompt_tokens: total_tokens,
                    total_tokens,
                    estimated_cost: 0.0, // Local inference is free
                };

                let routing_info = RoutingInfo {
                    selected_provider: LLMProviderType::Ollama,
                    routing_strategy: RoutingStrategy::ModelSpecific(request.model.clone()),
                    latency_ms: start_time.elapsed().as_millis() as u64,
                    retry_count: 0,
                    fallback_used: false,
                    provider_used: LLMProviderType::Ollama,
                    total_latency_ms: start_time.elapsed().as_millis() as u64,
                    provider_latency_ms: start_time.elapsed().as_millis() as u64,
                };

                Ok(EmbeddingsResponse {
                    id: request_id,
                    object: "list".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: request.model.clone(),
                    data: embeddings_data,
                    usage,
                    provider: LLMProviderType::Ollama,
                    routing_info,
                })
            }
        }
    }
}

impl OllamaClient {
    /// Get embeddings for a single text
    async fn get_single_embedding(
        &self,
        model: &str,
        text: &str,
        api_key: &str,
    ) -> LLMResult<Vec<f64>> {
        let ollama_request = OllamaEmbeddingsRequest {
            model: model.to_string(),
            prompt: text.to_string(),
            options: None,
            keep_alive: Some(self.config.keep_alive.clone()),
        };

        let url = format!("{}/api/embeddings", self.config.base_url);
        let headers = self.build_headers(api_key)?;

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Embeddings request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as Ollama error
            if let Ok(ollama_error) = serde_json::from_str::<OllamaError>(&error_text) {
                return Err(LLMError::Provider(ollama_error.error));
            }

            return Err(LLMError::Provider(format!(
                "Embeddings request failed with status {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaEmbeddingsResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(format!("Failed to parse embeddings response: {}", e)))?;

        Ok(ollama_response.embedding)
    }
}

/// Stream adapter for Ollama streaming responses
struct OllamaStreamAdapter {
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    model: String,
    buffer: String,
}

impl OllamaStreamAdapter {
    fn new(
        stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
        model: String,
    ) -> Self {
        Self {
            inner: Box::pin(stream),
            model,
            buffer: String::new(),
        }
    }

    fn parse_chunk(&self, chunk_data: &str) -> Option<LLMResult<StreamingChunk>> {
        if chunk_data.trim().is_empty() {
            return None;
        }

        match serde_json::from_str::<OllamaStreamingChunk>(chunk_data) {
            Ok(ollama_chunk) => {
                let streaming_chunk = StreamingChunk {
                    id: uuid::Uuid::new_v4().to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: self.model.clone(),
                    choices: vec![crate::llm::StreamingChoice {
                        index: 0,
                        delta: ollama_chunk.message.into(),
                        finish_reason: if ollama_chunk.done {
                            Some("stop".to_string())
                        } else {
                            None
                        },
                    }],
                    provider: LLMProviderType::Ollama,
                };
                Some(Ok(streaming_chunk))
            }
            Err(e) => {
                error!("Failed to parse Ollama streaming chunk: {}", e);
                Some(Err(LLMError::Parse(format!(
                    "Invalid streaming chunk: {}",
                    e
                ))))
            }
        }
    }
}

impl Stream for OllamaStreamAdapter {
    type Item = LLMResult<StreamingChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let chunk_str = String::from_utf8_lossy(&bytes);
                    self.buffer.push_str(&chunk_str);

                    // Process complete lines
                    while let Some(newline_pos) = self.buffer.find('\n') {
                        let line = self.buffer.drain(..=newline_pos).collect::<String>();
                        let line = line.trim();

                        if let Some(result) = self.parse_chunk(line) {
                            return Poll::Ready(Some(result));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(LLMError::Network(format!(
                        "Stream error: {}",
                        e
                    )))));
                }
                Poll::Ready(None) => {
                    // Process any remaining data in buffer
                    if !self.buffer.trim().is_empty() {
                        let remaining = self.buffer.clone();
                        self.buffer.clear();
                        if let Some(result) = self.parse_chunk(&remaining) {
                            return Poll::Ready(Some(result));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Unpin for OllamaStreamAdapter {}

impl OllamaClient {
    /// Create ModelInfo for the configured default model
    fn create_model_info_for_configured_default(&self) -> ModelInfo {
        self.create_model_info_from_id(&self.config.default_model)
    }

    /// Create ModelInfo from a model ID
    fn create_model_info_from_id(&self, model_id: &str) -> ModelInfo {
        let capabilities = determine_model_capabilities_from_name(model_id);
        let (context_window, max_output_tokens) = estimate_model_capabilities_from_name(model_id);

        ModelInfo {
            id: model_id.to_string(),
            name: format!("Ollama: {}", model_id),
            provider: LLMProviderType::Ollama,
            context_window,
            max_output_tokens,
            supports_streaming: true,
            supports_function_calling: supports_function_calling(model_id),
            cost_per_input_token: 0.0, // Local inference is free
            cost_per_output_token: 0.0,
            capabilities,
            parameter_restrictions: std::collections::HashMap::new(),
        }
    }
}

/// Estimate model capabilities from just the model name (for configured defaults)
fn estimate_model_capabilities_from_name(name: &str) -> (u32, u32) {
    // Extract parameter size hints from model name
    if name.contains("3b") || name.contains("3B") {
        (8192, 4096)
    } else if name.contains("7b") || name.contains("7B") {
        (16384, 8192)
    } else if name.contains("13b") || name.contains("13B") {
        (32768, 16384)
    } else if name.contains("70b") || name.contains("70B") {
        (65536, 32768)
    } else {
        (8192, 4096) // Conservative default
    }
}

/// Helper function to extract system message from messages
fn extract_system_message(
    messages: Vec<OllamaChatMessage>,
) -> (Option<String>, Vec<OllamaChatMessage>) {
    let mut system_message = None;
    let mut filtered_messages = Vec::new();

    for message in messages {
        if message.role == "system" {
            system_message = Some(message.content);
        } else {
            filtered_messages.push(message);
        }
    }

    (system_message, filtered_messages)
}

/// Convert Ollama model info to our ModelInfo format
fn convert_ollama_model_to_info(ollama_model: OllamaModelInfo) -> ModelInfo {
    // Extract parameter size and estimate context window
    let (context_window, max_output_tokens) =
        estimate_model_capabilities(&ollama_model.name, &ollama_model.details.parameter_size);

    let capabilities = determine_model_capabilities(&ollama_model.name);

    ModelInfo {
        id: ollama_model.name.clone(),
        name: format!(
            "{} ({})",
            ollama_model.name, ollama_model.details.parameter_size
        ),
        provider: LLMProviderType::Ollama,
        context_window,
        max_output_tokens,
        supports_streaming: true, // Most Ollama models support streaming
        supports_function_calling: supports_function_calling(&ollama_model.name),
        cost_per_input_token: 0.0, // Local inference is free
        cost_per_output_token: 0.0,
        capabilities,
        parameter_restrictions: HashMap::new(),
    }
}

/// Estimate model capabilities based on name and parameter size
fn estimate_model_capabilities(name: &str, parameter_size: &str) -> (u32, u32) {
    // Extract parameter count
    let param_count = parameter_size
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(7);

    // Estimate context window based on model name and size
    let context_window = if name.contains("codellama") {
        16384
    } else if name.contains("llama3") {
        8192
    } else if name.contains("gemma") {
        8192
    } else if param_count >= 70 {
        4096 // Large models might have smaller context windows due to memory constraints
    } else if param_count >= 13 {
        8192
    } else {
        4096
    };

    let max_output_tokens = context_window / 2; // Conservative estimate

    (context_window, max_output_tokens)
}

/// Determine model capabilities based on name patterns
fn determine_model_capabilities_from_name(name: &str) -> Vec<crate::llm::traits::ModelCapability> {
    use crate::llm::traits::ModelCapability;

    let mut capabilities = vec![
        ModelCapability::TextGeneration,
        ModelCapability::ConversationalAI,
    ];

    // Code generation models
    if name.contains("code") || name.contains("phi") || name.contains("granite") {
        capabilities.push(ModelCapability::CodeGeneration);
    }

    // Reasoning models
    if name.contains("llama") || name.contains("gemma") || name.contains("mistral") {
        capabilities.push(ModelCapability::Reasoning);
    }

    // Embedding models
    if name.contains("embed") {
        capabilities.push(ModelCapability::Embedding);
        // Embedding models typically don't do text generation
        capabilities.retain(|cap| {
            !matches!(
                cap,
                ModelCapability::TextGeneration | ModelCapability::ConversationalAI
            )
        });
    }

    capabilities
}

/// Legacy function for backward compatibility
pub fn determine_model_capabilities(name: &str) -> Vec<crate::llm::traits::ModelCapability> {
    determine_model_capabilities_from_name(name)
}

/// Simple token estimation for embeddings
fn estimate_tokens(text: &str) -> u32 {
    // Rough estimation: ~4 characters per token for English text
    (text.len() as f32 / 4.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMessage, FunctionDefinition, LLMRequest, MessageRole};
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_tool_calling_support_detection() {
        // Test models that should support function calling
        assert!(supports_function_calling("llama3.1:7b"));
        assert!(supports_function_calling("mistral:8x7b"));
        assert!(supports_function_calling("functionary:7b"));
        assert!(supports_function_calling("hermes:13b"));
        assert!(supports_function_calling("qwen2.5:7b"));
        assert!(supports_function_calling("phi3.5:mini"));

        // Test models that should NOT support function calling
        assert!(!supports_function_calling("llama2:7b"));
        assert!(!supports_function_calling("mistral:7b"));
        assert!(!supports_function_calling("codellama:7b"));
        assert!(!supports_function_calling("gemma:7b"));
    }

    #[test]
    fn test_convert_request_with_tools() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(config);

        let tool = FunctionDefinition {
            name: "get_weather".to_string(),
            description: "Get weather information for a location".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The location to get weather for"
                    }
                },
                "required": ["location"]
            }),
        };

        let request = LLMRequest {
            id: Uuid::new_v4(),
            model: "llama3.1:7b".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "What's the weather like in Paris?".to_string(),
                name: None,
                function_call: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            functions: Some(vec![tool]),
            stream: Some(false),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            function_call: None,
            user: None,
            metadata: HashMap::new(),
        };

        let ollama_request = client.convert_request(&request).unwrap();

        // Verify tools are converted properly
        assert!(ollama_request.tools.is_some());
        let tools = ollama_request.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "get_weather");
        assert_eq!(
            tools[0].function.description,
            "Get weather information for a location"
        );
    }

    #[test]
    fn test_message_conversion_with_tool_calls() {
        let function_call = crate::llm::FunctionCall {
            name: "get_weather".to_string(),
            arguments: r#"{"location": "Paris"}"#.to_string(),
        };

        let chat_message = ChatMessage {
            role: MessageRole::Assistant,
            content: "I'll get the weather for Paris.".to_string(),
            name: None,
            function_call: Some(function_call),
        };

        let ollama_message: OllamaChatMessage = (&chat_message).into();

        // Verify tool calls are converted
        assert!(ollama_message.tool_calls.is_some());
        let tool_calls = ollama_message.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "get_weather");

        // Convert back and verify
        let converted_back: ChatMessage = ollama_message.into();
        assert!(converted_back.function_call.is_some());
        let fc = converted_back.function_call.unwrap();
        assert_eq!(fc.name, "get_weather");
    }

    #[test]
    fn test_model_info_function_calling_support() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(config);

        // Test model that supports function calling
        let model_info = client.create_model_info_from_id("llama3.1:7b");
        assert!(model_info.supports_function_calling);

        // Test model that doesn't support function calling
        let model_info = client.create_model_info_from_id("llama2:7b");
        assert!(!model_info.supports_function_calling);
    }
}
