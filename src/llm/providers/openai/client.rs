//! OpenAI provider client implementation
//! This module contains the actual client that makes requests to OpenAI's API

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use tracing::{debug, error};

use std::time::Duration;

use crate::llm::{
    LLMError, LLMRequest, LLMResponse, LLMResult, StreamingChunk,
    Choice, LLMProviderType, RoutingInfo, RoutingStrategy,
    sse::{response_to_sse_stream, openai::openai_event_to_chunk}
};

use crate::llm::traits::{
    LLMProviderClient, ModelInfo, ProviderConfigRequirements, CostCalculator, CostBreakdown
};

use super::types::{
    OpenAIRequest, OpenAIResponse, OpenAIUsage, OpenAIChatMessage, OpenAIError
};
use super::config::{OpenAIConfig, get_config_requirements, get_available_models, is_o4_model};

/// OpenAI provider client
pub struct OpenAIClient {
    client: Client,
    config: OpenAIConfig,
}

impl OpenAIClient {
    /// Create a new OpenAI client with configuration
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a new OpenAI client with default configuration
    pub fn with_api_key(api_key: String) -> Self {
        let mut config = OpenAIConfig::default();
        config.api_key = api_key;
        Self::new(config)
    }

    /// Build HTTP headers for requests
    fn build_headers(&self) -> LLMResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .map_err(|e| LLMError::Internal(format!("Invalid API key format: {}", e)))?
        );

        // Add organization header if provided
        if let Some(org) = &self.config.organization {
            headers.insert(
                "OpenAI-Organization",
                HeaderValue::from_str(org)
                    .map_err(|e| LLMError::Internal(format!("Invalid organization format: {}", e)))?
            );
        }

        // Add project header if provided
        if let Some(project) = &self.config.project {
            headers.insert(
                "OpenAI-Project",
                HeaderValue::from_str(project)
                    .map_err(|e| LLMError::Internal(format!("Invalid project format: {}", e)))?
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

    /// Convert our internal request format to OpenAI's format
    fn convert_request(&self, request: &LLMRequest) -> LLMResult<OpenAIRequest> {
        let messages: Vec<OpenAIChatMessage> = request.messages.iter()
            .map(|msg| msg.into())
            .collect();

        // Handle model-specific parameter requirements
        let (max_tokens_field, temperature) = if is_o4_model(&request.model) {
            // o4 models require max_completion_tokens and temperature=1.0
            (request.max_tokens, Some(1.0))
        } else {
            // Regular models use max_tokens and allow custom temperature
            (request.max_tokens, request.temperature)
        };

        let mut openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            temperature,
            max_tokens: None,
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

        // Set the appropriate max tokens field
        if is_o4_model(&request.model) {
            openai_request.max_completion_tokens = max_tokens_field;
        } else {
            openai_request.max_tokens = max_tokens_field;
        }

        Ok(openai_request)
    }

    /// Convert OpenAI response to our internal format
    fn convert_response(&self, response: OpenAIResponse) -> LLMResult<LLMResponse> {
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
            estimated_cost: self.calculate_cost(&response.usage, &response.model),
        };

        Ok(LLMResponse {
            id: response.id,
            object: response.object,
            created: response.created,
            model: response.model,
            choices,
            usage,
            provider: LLMProviderType::OpenAI,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::OpenAI,
                routing_strategy: RoutingStrategy::ModelSpecific("openai".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
                provider_used: LLMProviderType::OpenAI,
                total_latency_ms: 0,
                provider_latency_ms: 0,
            },
        })
    }

    /// Calculate cost for OpenAI usage
    fn calculate_cost(&self, usage: &OpenAIUsage, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.prompt_tokens as f64 * input_cost) + (usage.completion_tokens as f64 * output_cost)
        } else {
            // Fallback to GPT-4 pricing if model not found
            (usage.prompt_tokens as f64 * 0.00003) + (usage.completion_tokens as f64 * 0.00006)
        }
    }

    /// Handle error responses from OpenAI
    fn handle_error_response(&self, status_code: u16, error_text: &str) -> LLMError {
        // Try to parse as OpenAI error format
        if let Ok(openai_error) = serde_json::from_str::<OpenAIError>(error_text) {
            match status_code {
                401 => LLMError::AuthenticationFailed(openai_error.error.message),
                429 => LLMError::RateLimitExceeded(openai_error.error.message),
                400 => LLMError::InvalidRequest(openai_error.error.message),
                _ => LLMError::Internal(format!(
                    "OpenAI API error ({}): {}",
                    status_code, openai_error.error.message
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
}

#[async_trait]
impl LLMProviderClient for OpenAIClient {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        // Update config with provided API key if different
        let mut client_config = self.config.clone();
        if api_key != self.config.api_key {
            client_config.api_key = api_key.to_string();
        }
        let temp_client = OpenAIClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let openai_request = temp_client.convert_request(request)?;

        let request_url = format!("{}/chat/completions", temp_client.config.base_url);
        
        debug!("OpenAI API Request: URL={}, Model={}", request_url, request.model);
        debug!("API key: {}...", &api_key[..8.min(api_key.len())]);

        let response = temp_client.client
            .post(&request_url)
            .headers(headers)
            .json(&openai_request)
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

            error!("OpenAI API Error: {} - {}", status, error_text);
            return Err(temp_client.handle_error_response(status.as_u16(), &error_text));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Serialization(e.to_string()))?;

        temp_client.convert_response(openai_response)
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
        let temp_client = OpenAIClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let mut openai_request = temp_client.convert_request(&request)?;
        
        // Enable streaming for this request
        openai_request.stream = Some(true);

        let request_url = format!("{}/chat/completions", temp_client.config.base_url);
        


        let response = temp_client.client
            .post(&request_url)
            .headers(headers)
            .json(&openai_request)
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

        // Convert response to SSE stream and parse OpenAI events
        let sse_stream = response_to_sse_stream(response);
        let chunk_stream = sse_stream.filter_map(move |sse_result| async move {
            match sse_result {
                Ok(sse_event) => {
                    match openai_event_to_chunk(&sse_event) {
                        Ok(Some(chunk)) => Some(Ok(chunk)),
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
        LLMProviderType::OpenAI
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let mut client_config = self.config.clone();
        client_config.api_key = api_key.to_string();
        let temp_client = OpenAIClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let request_url = format!("{}/models", temp_client.config.base_url);

        let response = temp_client.client
            .get(&request_url)
            .headers(headers)
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
}

impl CostCalculator for OpenAIClient {
    fn calculate_cost(&self, usage: &crate::llm::TokenUsage, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.prompt_tokens as f64 * input_cost) + (usage.completion_tokens as f64 * output_cost)
        } else {
            // Fallback to GPT-4 pricing
            (usage.prompt_tokens as f64 * 0.00003) + (usage.completion_tokens as f64 * 0.00006)
        }
    }

    fn estimate_cost(&self, input_tokens: u32, estimated_output_tokens: u32, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (input_tokens as f64 * input_cost) + (estimated_output_tokens as f64 * output_cost)
        } else {
            // Fallback to GPT-4 pricing
            (input_tokens as f64 * 0.00003) + (estimated_output_tokens as f64 * 0.00006)
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
            // Fallback to GPT-4 pricing
            let input_cost_total = usage.prompt_tokens as f64 * 0.00003;
            let output_cost_total = usage.completion_tokens as f64 * 0.00006;
            
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
    fn test_openai_client_creation() {
        let config = OpenAIConfig::default();
        let client = OpenAIClient::new(config);
        assert_eq!(client.provider_type(), LLMProviderType::OpenAI);
    }

    #[test]
    fn test_convert_request_regular_model() {
        let client = OpenAIClient::with_api_key("test-key".to_string());
        let request = LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                function_call: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
            user: None,
            project_id: None,
        };

        let openai_request = client.convert_request(&request).unwrap();
        assert_eq!(openai_request.model, "gpt-4");
        assert_eq!(openai_request.temperature, Some(0.7));
        assert_eq!(openai_request.max_tokens, Some(100));
        assert_eq!(openai_request.max_completion_tokens, None);
    }

    #[test]
    fn test_convert_request_o4_model() {
        let client = OpenAIClient::with_api_key("test-key".to_string());
        let request = LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: "o4-mini-2025-04-16".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                function_call: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
            user: None,
            project_id: None,
        };

        let openai_request = client.convert_request(&request).unwrap();
        assert_eq!(openai_request.model, "o4-mini-2025-04-16");
        assert_eq!(openai_request.temperature, Some(1.0)); // Should be forced to 1.0
        assert_eq!(openai_request.max_tokens, None);
        assert_eq!(openai_request.max_completion_tokens, Some(100));
    }

    #[test]
    fn test_model_support() {
        let client = OpenAIClient::with_api_key("test-key".to_string());
        assert!(client.supports_model("gpt-4"));
        assert!(client.supports_model("o4-mini-2025-04-16"));
        assert!(!client.supports_model("claude-3"));
    }
}