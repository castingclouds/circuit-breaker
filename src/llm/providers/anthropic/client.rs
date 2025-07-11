//! Anthropic provider client implementation
//! This module contains the actual client that makes requests to Anthropic's API

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error};

use crate::llm::{
    sse::{anthropic::anthropic_event_to_chunk, response_to_sse_stream},
    Choice, EmbeddingsRequest, EmbeddingsResponse, LLMError, LLMProviderType, LLMRequest,
    LLMResponse, LLMResult, MessageRole, RoutingInfo, RoutingStrategy, StreamingChunk,
};

use crate::llm::traits::{
    CostBreakdown, CostCalculator, LLMProviderClient, ModelInfo, ProviderConfigRequirements,
};

use super::config::{get_available_models, get_config_requirements, AnthropicConfig};
use super::types::{
    AnthropicError, AnthropicMessage, AnthropicRequest, AnthropicResponse, AnthropicUsage,
};

/// Anthropic provider client
pub struct AnthropicClient {
    client: Client,
    config: AnthropicConfig,
}

impl AnthropicClient {
    /// Create a new Anthropic client with configuration
    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a new Anthropic client with default configuration
    pub fn with_api_key(api_key: String) -> Self {
        let mut config = AnthropicConfig::default();
        config.api_key = api_key;
        Self::new(config)
    }

    /// Build HTTP headers for requests
    fn build_headers(&self) -> LLMResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| LLMError::Internal(format!("Invalid API key format: {}", e)))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&self.config.api_version)
                .map_err(|e| LLMError::Internal(format!("Invalid API version format: {}", e)))?,
        );

        // Add custom headers
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

    /// Convert our internal request format to Anthropic's format
    fn convert_request(&self, request: &LLMRequest) -> LLMResult<AnthropicRequest> {
        let mut system_prompt = None;
        let mut messages = Vec::new();

        // Anthropic handles system messages differently
        for msg in &request.messages {
            match msg.role {
                MessageRole::System => {
                    system_prompt = Some(msg.content.clone());
                }
                _ => {
                    messages.push(AnthropicMessage::from(msg));
                }
            }
        }

        // Convert function definitions to Anthropic tools format
        let tools = request.functions.as_ref().map(|functions| {
            functions
                .iter()
                .map(|func| super::types::AnthropicTool {
                    name: func.name.clone(),
                    description: func.description.clone(),
                    input_schema: func.parameters.clone(),
                })
                .collect()
        });

        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(1024), // Anthropic requires max_tokens
            temperature: request.temperature.map(|t| t as f64),
            top_p: request.top_p.map(|p| p as f64),
            top_k: None,
            stop_sequences: request.stop.clone(),
            stream: Some(false), // Force non-streaming for regular chat_completion
            system: system_prompt,
            tools,
        };

        Ok(anthropic_request)
    }

    /// Convert Anthropic response to our internal format
    fn convert_response(&self, response: AnthropicResponse) -> LLMResult<LLMResponse> {
        let choice = Choice {
            index: 0,
            message: response.to_chat_message(),
            finish_reason: response.stop_reason.clone(),
        };

        let usage = crate::llm::TokenUsage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            estimated_cost: self.calculate_cost(&response.usage, &response.model),
        };

        Ok(LLMResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: response.model,
            choices: vec![choice],
            usage,
            provider: LLMProviderType::Anthropic,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::Anthropic,
                routing_strategy: RoutingStrategy::ModelSpecific("anthropic".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
                provider_used: LLMProviderType::Anthropic,
                total_latency_ms: 0,
                provider_latency_ms: 0,
            },
        })
    }

    /// Calculate cost for Anthropic usage
    fn calculate_cost(&self, usage: &AnthropicUsage, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.input_tokens as f64 * input_cost) + (usage.output_tokens as f64 * output_cost)
        } else {
            // Fallback to Claude 3 Sonnet pricing if model not found
            (usage.input_tokens as f64 * 0.000003) + (usage.output_tokens as f64 * 0.000015)
        }
    }

    /// Handle error responses from Anthropic
    fn handle_error_response(&self, status_code: u16, error_text: &str) -> LLMError {
        // Try to parse as Anthropic error format
        if let Ok(anthropic_error) = serde_json::from_str::<AnthropicError>(error_text) {
            match status_code {
                401 => LLMError::AuthenticationFailed(anthropic_error.error.message),
                429 => LLMError::RateLimitExceeded(anthropic_error.error.message),
                400 => LLMError::InvalidRequest(anthropic_error.error.message),
                _ => LLMError::Internal(format!(
                    "Anthropic API error ({}): {}",
                    status_code, anthropic_error.error.message
                )),
            }
        } else {
            // Fallback for non-JSON errors
            match status_code {
                401 => LLMError::AuthenticationFailed(error_text.to_string()),
                429 => LLMError::RateLimitExceeded(error_text.to_string()),
                400 => LLMError::InvalidRequest(error_text.to_string()),
                _ => LLMError::Internal(format!("HTTP {}: {}", status_code, error_text)),
            }
        }
    }
}

#[async_trait]
impl LLMProviderClient for AnthropicClient {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        // Update config with provided API key if different
        let mut client_config = self.config.clone();
        if api_key != self.config.api_key {
            client_config.api_key = api_key.to_string();
        }
        let temp_client = AnthropicClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let anthropic_request = temp_client.convert_request(request)?;

        let request_url = format!("{}/v1/messages", temp_client.config.base_url);

        debug!(
            "Anthropic API Request: URL={}, Model={}",
            request_url, request.model
        );
        debug!("API key: {}...", &api_key[..8.min(api_key.len())]);

        let response = temp_client
            .client
            .post(&request_url)
            .headers(headers)
            .json(&anthropic_request)
            .timeout(Duration::from_secs(temp_client.config.timeout_seconds))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            error!("Anthropic API Error: {} - {}", status, error_text);
            return Err(temp_client.handle_error_response(status.as_u16(), &error_text));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        temp_client.convert_response(anthropic_response)
    }

    async fn chat_completion_stream(
        &self,
        request: LLMRequest,
        api_key: String,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        // Update config with provided API key if different
        let mut client_config = self.config.clone();
        if api_key != self.config.api_key {
            client_config.api_key = api_key.clone();
        }
        let temp_client = AnthropicClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let mut anthropic_request = temp_client.convert_request(&request)?;

        // Enable streaming for this request
        anthropic_request.stream = Some(true);

        let request_url = format!("{}/v1/messages", temp_client.config.base_url);

        let response = temp_client
            .client
            .post(&request_url)
            .headers(headers)
            .json(&anthropic_request)
            .timeout(Duration::from_secs(temp_client.config.timeout_seconds))
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(temp_client.handle_error_response(status.as_u16(), &error_text));
        }

        // Convert response to SSE stream and parse Anthropic events
        let request_id = request.id.to_string();
        let model = request.model;

        let sse_stream = response_to_sse_stream(response);
        let chunk_stream = sse_stream.filter_map(move |sse_result| {
            let request_id = request_id.clone();
            let model = model.clone();
            async move {
                match sse_result {
                    Ok(sse_event) => {
                        match anthropic_event_to_chunk(&sse_event, &request_id, &model) {
                            Ok(Some(chunk)) => Some(Ok(chunk)),
                            Ok(None) => None, // Skip empty chunks
                            Err(e) => Some(Err(e)),
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Ok(Box::new(Box::pin(chunk_stream)))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Anthropic
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let mut client_config = self.config.clone();
        client_config.api_key = api_key.to_string();
        let temp_client = AnthropicClient::new(client_config);

        let headers = temp_client.build_headers()?;

        // Use a simple health check request
        let health_request = json!({
            "model": "claude-3-haiku-20240307",
            "messages": [{"role": "user", "content": "ping"}],
            "max_tokens": 1
        });

        let request_url = format!("{}/v1/messages", temp_client.config.base_url);

        let response = temp_client
            .client
            .post(&request_url)
            .headers(headers)
            .json(&health_request)
            .timeout(Duration::from_secs(10)) // Shorter timeout for health checks
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        Ok(response.status().is_success())
    }

    fn get_available_models(&self) -> Vec<ModelInfo> {
        get_available_models()
    }

    fn supports_model(&self, model: &str) -> bool {
        let models = get_available_models();
        models.iter().any(|m| m.id == model)
    }

    fn get_config_requirements(&self) -> ProviderConfigRequirements {
        get_config_requirements()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn embeddings(
        &self,
        _request: &EmbeddingsRequest,
        _api_key: &str,
    ) -> LLMResult<EmbeddingsResponse> {
        // TODO: Implement Anthropic embeddings support (if they provide this service)
        Err(LLMError::Provider(
            "Embeddings not yet implemented for Anthropic provider".to_string(),
        ))
    }
}

impl CostCalculator for AnthropicClient {
    fn calculate_cost(&self, usage: &crate::llm::TokenUsage, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.prompt_tokens as f64 * input_cost)
                + (usage.completion_tokens as f64 * output_cost)
        } else {
            // Fallback to Claude 3 Sonnet pricing
            (usage.prompt_tokens as f64 * 0.000003) + (usage.completion_tokens as f64 * 0.000015)
        }
    }

    fn estimate_cost(&self, input_tokens: u32, estimated_output_tokens: u32, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (input_tokens as f64 * input_cost) + (estimated_output_tokens as f64 * output_cost)
        } else {
            // Fallback to Claude 3 Sonnet pricing
            (input_tokens as f64 * 0.000003) + (estimated_output_tokens as f64 * 0.000015)
        }
    }

    fn get_cost_breakdown(&self, usage: &crate::llm::TokenUsage, model: &str) -> CostBreakdown {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            let input_cost_total = usage.prompt_tokens as f64 * input_cost;
            let output_cost_total = usage.completion_tokens as f64 * output_cost;

            CostBreakdown {
                input_cost: input_cost_total,
                output_cost: output_cost_total,
                total_cost: input_cost_total + output_cost_total,
                currency: "USD".to_string(),
            }
        } else {
            // Fallback to Claude 3 Sonnet pricing
            let input_cost_total = usage.prompt_tokens as f64 * 0.000003;
            let output_cost_total = usage.completion_tokens as f64 * 0.000015;

            CostBreakdown {
                input_cost: input_cost_total,
                output_cost: output_cost_total,
                total_cost: input_cost_total + output_cost_total,
                currency: "USD".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMessage, MessageRole};

    #[test]
    fn test_anthropic_client_creation() {
        let config = AnthropicConfig::default();
        let client = AnthropicClient::new(config);
        assert_eq!(client.provider_type(), LLMProviderType::Anthropic);
    }

    #[test]
    fn test_convert_request() {
        let client = AnthropicClient::with_api_key("test-key".to_string());
        let request = crate::llm::LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: "claude-3-sonnet-20240229".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "You are a helpful assistant".to_string(),
                    name: None,
                    function_call: None,
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    name: None,
                    function_call: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
            functions: None,
            function_call: None,
            user: None,
            metadata: std::collections::HashMap::new(),
        };

        let anthropic_request = client.convert_request(&request).unwrap();
        assert_eq!(anthropic_request.model, "claude-3-sonnet-20240229");
        assert_eq!(
            anthropic_request.system,
            Some("You are a helpful assistant".to_string())
        );
        assert_eq!(anthropic_request.messages.len(), 1);
        assert_eq!(anthropic_request.messages[0].content, "Hello");
    }

    #[test]
    fn test_model_support() {
        let client = AnthropicClient::with_api_key("test-key".to_string());
        assert!(client.supports_model("claude-3-sonnet-20240229"));
        assert!(client.supports_model("claude-sonnet-4-20250514"));
        assert!(!client.supports_model("gpt-4"));
    }
}
