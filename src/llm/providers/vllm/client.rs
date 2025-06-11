//! vLLM provider client implementation
//! This module contains the actual client that makes requests to vLLM's OpenAI-compatible API

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use tracing::{debug, error, info, warn};
use std::time::Duration;

use crate::llm::{
    LLMError, LLMRequest, LLMResponse, LLMResult, StreamingChunk,
    Choice, LLMProviderType, RoutingInfo, RoutingStrategy,
    EmbeddingsRequest, EmbeddingsResponse, EmbeddingsInput,
    sse::{response_to_sse_stream, openai::openai_event_to_chunk}
};

use crate::llm::traits::{
    LLMProviderClient, ModelInfo, ProviderConfigRequirements, CostCalculator, CostBreakdown
};

use super::types::{
    VLLMRequest, VLLMResponse, VLLMModelsResponse,
    VLLMEmbeddingsRequest, VLLMEmbeddingsResponse
};
use super::config::{VLLMConfig, get_config_requirements, get_default_models};

/// vLLM provider client
pub struct VLLMClient {
    client: Client,
    config: VLLMConfig,
}

impl VLLMClient {
    /// Create a new vLLM client with configuration
    pub fn new(config: VLLMConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    /// Create a new vLLM client with default configuration
    pub fn with_base_url(base_url: String) -> Self {
        let mut config = VLLMConfig::default();
        config.base_url = base_url;
        Self::new(config)
    }

    /// Build HTTP headers for requests
    fn build_headers(&self, api_key: &str) -> LLMResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add API key if provided (vLLM can optionally require authentication)
        if !api_key.is_empty() {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| LLMError::Internal(format!("Invalid API key format: {}", e)))?
            );
        } else if let Some(ref config_api_key) = self.config.api_key {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", config_api_key))
                    .map_err(|e| LLMError::Internal(format!("Invalid API key format: {}", e)))?
            );
        }

        // Add custom headers
        for (key, value) in &self.config.custom_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| LLMError::Internal(format!("Invalid header key: {}", e)))?;
            headers.insert(
                header_name,
                HeaderValue::from_str(value)
                    .map_err(|e| LLMError::Internal(format!("Invalid header value: {}", e)))?
            );
        }

        Ok(headers)
    }

    /// Convert our internal request format to vLLM's OpenAI-compatible format
    fn convert_request(&self, request: &LLMRequest) -> LLMResult<VLLMRequest> {
        // Convert messages to OpenAI format
        let messages: Vec<crate::llm::providers::openai::types::OpenAIChatMessage> = request.messages.iter()
            .map(|msg| msg.into())
            .collect();

        let vllm_request = VLLMRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            max_completion_tokens: None,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop.clone(),
            stream: Some(false), // Force non-streaming for regular chat_completion
            user: request.user.clone(),
            response_format: None,
            tools: None,
            tool_choice: None,
        };

        Ok(vllm_request)
    }

    /// Convert vLLM response to our internal format
    fn convert_response(&self, response: VLLMResponse) -> LLMResult<LLMResponse> {
        let choices: Vec<Choice> = response.choices.into_iter()
            .map(|choice| Choice {
                index: choice.index,
                message: choice.message,
                finish_reason: choice.finish_reason,
            })
            .collect();

        let usage = crate::llm::TokenUsage {
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
            estimated_cost: 0.0, // Local inference is free
        };

        Ok(LLMResponse {
            id: response.id,
            object: response.object,
            created: response.created,
            model: response.model,
            choices,
            usage,
            provider: LLMProviderType::VLLM,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::VLLM,
                routing_strategy: RoutingStrategy::ModelSpecific("vllm".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
                provider_used: LLMProviderType::VLLM,
                total_latency_ms: 0,
                provider_latency_ms: 0,
            },
        })
    }

    /// Handle error responses from vLLM
    fn handle_error_response(&self, status_code: u16, error_text: &str) -> LLMError {
        // Try to parse as vLLM/OpenAI error format
        if let Ok(vllm_error) = serde_json::from_str::<super::types::VLLMError>(error_text) {
            match status_code {
                401 => LLMError::AuthenticationFailed(vllm_error.error.message),
                429 => LLMError::RateLimitExceeded(vllm_error.error.message),
                400 => LLMError::InvalidRequest(vllm_error.error.message),
                _ => LLMError::Internal(format!(
                    "vLLM API error ({}): {}",
                    status_code, vllm_error.error.message
                )),
            }
        } else {
            // Fallback for non-JSON errors
            match status_code {
                401 => LLMError::AuthenticationFailed(error_text.to_string()),
                429 => LLMError::RateLimitExceeded(error_text.to_string()),
                400 => LLMError::InvalidRequest(error_text.to_string()),
                _ => LLMError::Internal(format!(
                    "HTTP {}: {}",
                    status_code, error_text
                )),
            }
        }
    }

    /// Fetch available models from vLLM
    pub async fn fetch_available_models(&self) -> LLMResult<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.config.base_url);
        let headers = self.build_headers("")?;

        debug!("Fetching available models from: {}", url);

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| LLMError::Network(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!("Model fetch failed with status {}: {}", status, error_text)));
        }

        let models_response: VLLMModelsResponse = response.json().await
            .map_err(|e| LLMError::Parse(format!("Failed to parse models response: {}", e)))?;

        // Convert vLLM model info to our ModelInfo format
        let model_infos = models_response.data.into_iter()
            .map(|model| {
                // Try to get predefined model info, otherwise create basic info
                super::config::get_model_info(&model.id).unwrap_or_else(|| ModelInfo {
                    id: model.id.clone(),
                    name: model.id.clone(),
                    provider: LLMProviderType::VLLM,
                    context_window: 4096,
                    max_output_tokens: 2048,
                    supports_streaming: true,
                    supports_function_calling: false,
                    cost_per_input_token: 0.0,
                    cost_per_output_token: 0.0,
                    capabilities: vec![crate::llm::traits::ModelCapability::TextGeneration],
                    parameter_restrictions: std::collections::HashMap::new(),
                })
            })
            .collect();

        Ok(model_infos)
    }

    /// Get available models asynchronously (for router dynamic fetching)
    pub async fn get_available_models_async(&self) -> Vec<ModelInfo> {
        match self.fetch_available_models().await {
            Ok(models) => {
                info!("Dynamically fetched {} models from vLLM server", models.len());
                models
            }
            Err(e) => {
                warn!("Failed to fetch models from vLLM server, using defaults: {}", e);
                get_default_models()
            }
        }
    }
}

#[async_trait]
impl LLMProviderClient for VLLMClient {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        // First check if the model is available on this vLLM server
        match self.fetch_available_models().await {
            Ok(available_models) => {
                let model_ids: Vec<String> = available_models.iter().map(|m| m.id.clone()).collect();
                if !model_ids.contains(&request.model) {
                    error!("Model '{}' not available on vLLM server. Available models: {:?}", request.model, model_ids);
                    return Err(LLMError::Provider(format!(
                        "Model '{}' is not available on this vLLM server. Available models: {}. This model may be available through other providers.",
                        request.model,
                        model_ids.join(", ")
                    )));
                }
            }
            Err(e) => {
                warn!("Could not fetch available models from vLLM server: {}. Proceeding with request.", e);
            }
        }

        let headers = self.build_headers(api_key)?;
        
        let vllm_request = self.convert_request(request)?;
        let request_url = format!("{}/v1/chat/completions", self.config.base_url);
        
        debug!("vLLM Chat Request: URL={}, Model={}", request_url, request.model);

        let response = self.client
            .post(&request_url)
            .headers(headers)
            .json(&vllm_request)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Check if this is a model not found error
            if error_text.contains("does not exist") {
                error!("vLLM Model Not Found: {}", error_text);
                return Err(LLMError::Provider(format!(
                    "Model '{}' not found on vLLM server. The model may not be loaded or may require authentication. Error: {}",
                    request.model, error_text
                )));
            }

            error!("vLLM API Error: {} - {}", status, error_text);
            return Err(self.handle_error_response(status.as_u16(), &error_text));
        }

        let vllm_response: VLLMResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        self.convert_response(vllm_response)
    }

    async fn chat_completion_stream(
        &self,
        request: LLMRequest,
        api_key: String,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        let headers = self.build_headers(&api_key)?;
        let mut vllm_request = self.convert_request(&request)?;
        
        // Enable streaming for this request
        vllm_request.stream = Some(true);

        let request_url = format!("{}/v1/chat/completions", self.config.base_url);

        let response = self.client
            .post(&request_url)
            .headers(headers)
            .json(&vllm_request)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(self.handle_error_response(status.as_u16(), &error_text));
        }

        // Convert response to SSE stream and parse OpenAI events (vLLM is compatible)
        let sse_stream = response_to_sse_stream(response);
        let chunk_stream = sse_stream.filter_map(move |sse_result| async move {
            match sse_result {
                Ok(sse_event) => {
                    match openai_event_to_chunk(&sse_event) {
                        Ok(Some(mut chunk)) => {
                            // Update provider to VLLM
                            chunk.provider = LLMProviderType::VLLM;
                            Some(Ok(chunk))
                        },
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => Some(Err(e)),
                    }
                }
                Err(e) => Some(Err(e)),
            }
        });

        Ok(Box::new(Box::pin(chunk_stream)))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::VLLM
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let headers = self.build_headers(api_key)?;
        let request_url = format!("{}/v1/models", self.config.base_url);

        let response = self.client
            .get(&request_url)
            .headers(headers)
            .timeout(Duration::from_secs(10)) // Shorter timeout for health checks
            .send()
            .await
            .map_err(|e| {
                warn!("vLLM health check failed: {}", e);
                LLMError::Network(e.to_string())
            })?;

        Ok(response.status().is_success())
    }

    fn get_available_models(&self) -> Vec<ModelInfo> {
        // For now, return default models to avoid async runtime conflicts
        // TODO: Make this trait method async to properly support dynamic fetching
        get_default_models()
    }

    fn supports_model(&self, model: &str) -> bool {
        let models = self.get_available_models();
        models.iter().any(|m| m.id == model)
    }

    fn get_config_requirements(&self) -> ProviderConfigRequirements {
        get_config_requirements()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn embeddings(&self, request: &EmbeddingsRequest, api_key: &str) -> LLMResult<EmbeddingsResponse> {
        let headers = self.build_headers(api_key)?;
        
        // Convert to vLLM embeddings request
        let input = match &request.input {
            EmbeddingsInput::Text(text) => vec![text.clone()],
            EmbeddingsInput::TextArray(texts) => texts.clone(),
        };

        let vllm_request = VLLMEmbeddingsRequest {
            model: request.model.clone(),
            input,
            encoding_format: Some("float".to_string()),
            user: request.user.clone(),
        };

        let request_url = format!("{}/v1/embeddings", self.config.base_url);
        
        debug!("vLLM Embeddings Request: URL={}, Model={}", request_url, request.model);

        let response = self.client
            .post(&request_url)
            .headers(headers)
            .json(&vllm_request)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Check if this is an "embeddings disabled" response
            if error_text.contains("Embedding API disabled") || error_text.contains("embeddings disabled") {
                info!("📋 vLLM embeddings are disabled on this server (this is normal for chat-focused deployments)");
                debug!("vLLM embeddings disabled response: {}", error_text);
                return Err(LLMError::Provider("Embeddings API is disabled on this vLLM server. This is a configuration choice, not an error.".to_string()));
            }

            // For other errors, log as actual errors
            error!("vLLM Embeddings API Error: {} - {}", status, error_text);
            return Err(self.handle_error_response(status.as_u16(), &error_text));
        }

        let vllm_response: VLLMEmbeddingsResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        // Convert to our internal format
        let data: Vec<crate::llm::EmbeddingData> = vllm_response.data.into_iter()
            .map(|embedding| crate::llm::EmbeddingData {
                index: embedding.index,
                embedding: embedding.embedding,
                object: embedding.object,
            })
            .collect();

        let usage = crate::llm::EmbeddingsUsage {
            prompt_tokens: vllm_response.usage.prompt_tokens,
            total_tokens: vllm_response.usage.total_tokens,
            estimated_cost: 0.0, // Local inference is free
        };

        let routing_info = RoutingInfo {
            selected_provider: LLMProviderType::VLLM,
            routing_strategy: RoutingStrategy::ModelSpecific("vllm".to_string()),
            latency_ms: 0,
            retry_count: 0,
            fallback_used: false,
            provider_used: LLMProviderType::VLLM,
            total_latency_ms: 0,
            provider_latency_ms: 0,
        };

        Ok(EmbeddingsResponse {
            id: request.id.to_string(),
            object: "list".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: request.model.clone(),
            data,
            usage,
            provider: LLMProviderType::VLLM,
            routing_info,
        })
    }
}

impl CostCalculator for VLLMClient {
    fn calculate_cost(&self, _usage: &crate::llm::TokenUsage, _model: &str) -> f64 {
        0.0 // Local inference is free
    }

    fn estimate_cost(&self, _input_tokens: u32, _estimated_output_tokens: u32, _model: &str) -> f64 {
        0.0 // Local inference is free
    }

    fn get_cost_breakdown(&self, _usage: &crate::llm::TokenUsage, _model: &str) -> CostBreakdown {
        CostBreakdown {
            input_cost: 0.0,
            output_cost: 0.0,
            total_cost: 0.0,
            currency: "USD".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMessage, MessageRole};

    #[test]
    fn test_vllm_client_creation() {
        let config = VLLMConfig::default();
        let client = VLLMClient::new(config);
        assert_eq!(client.provider_type(), LLMProviderType::VLLM);
    }

    #[test]
    fn test_model_support() {
        let client = VLLMClient::with_base_url("http://localhost:8000".to_string());
        assert!(client.supports_model("microsoft/DialoGPT-medium"));
        assert!(!client.supports_model("gpt-4"));
    }

    #[test]
    fn test_cost_calculation() {
        let client = VLLMClient::with_base_url("http://localhost:8000".to_string());
        let usage = crate::llm::TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            estimated_cost: 0.0,
        };
        
        // vLLM is local inference, so cost should always be 0
        assert_eq!(client.calculate_cost(&usage, "any-model"), 0.0);
        assert_eq!(client.estimate_cost(100, 50, "any-model"), 0.0);
        
        let breakdown = client.get_cost_breakdown(&usage, "any-model");
        assert_eq!(breakdown.total_cost, 0.0);
        assert_eq!(breakdown.input_cost, 0.0);
        assert_eq!(breakdown.output_cost, 0.0);
    }
}