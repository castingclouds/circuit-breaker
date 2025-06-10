//! Google provider client implementation
//! This module contains the actual client that makes requests to Google's Gemini API

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{header::HeaderMap, header::HeaderValue, header::CONTENT_TYPE, Client};
use tracing::{debug, error};
use std::time::Duration;

use crate::llm::{
    LLMError, LLMRequest, LLMResponse, LLMResult, StreamingChunk, StreamingChoice,
    Choice, LLMProviderType, RoutingInfo, RoutingStrategy, ChatMessage, MessageRole,
    EmbeddingsRequest, EmbeddingsResponse,
};

use crate::llm::traits::{
    LLMProviderClient, ModelInfo, ProviderConfigRequirements, CostCalculator, CostBreakdown
};

use super::types::{
    GoogleRequest, GoogleResponse, GoogleUsageMetadata, GoogleGenerationConfig,
    GoogleError, convert_conversation_history
};
use super::config::{GoogleConfig, get_config_requirements, get_available_models};

/// Google provider client
pub struct GoogleClient {
    client: Client,
    config: GoogleConfig,
}

impl GoogleClient {
    /// Create a new Google client with configuration
    pub fn new(config: GoogleConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Create a new Google client with default configuration
    pub fn with_api_key(api_key: String) -> Self {
        let mut config = GoogleConfig::default();
        config.api_key = api_key;
        Self::new(config)
    }

    /// Build HTTP headers for requests
    fn build_headers(&self) -> LLMResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

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

    /// Convert our internal request format to Google's format
    fn convert_request(&self, request: &LLMRequest) -> LLMResult<GoogleRequest> {
        let contents = convert_conversation_history(&request.messages);

        let generation_config = GoogleGenerationConfig {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_output_tokens: request.max_tokens,
            candidate_count: Some(1),
            stop_sequences: request.stop.clone(),
        };

        let google_request = GoogleRequest {
            contents,
            generation_config: Some(generation_config),
            safety_settings: Some(super::config::get_default_safety_settings()),
            tools: None, // TODO: Implement function calling support
        };

        Ok(google_request)
    }

    /// Convert Google response to our internal format
    fn convert_response(&self, response: GoogleResponse, model: &str) -> LLMResult<LLMResponse> {
        let choice = Choice {
            index: 0,
            message: response.to_chat_message(),
            finish_reason: response.get_finish_reason(),
        };

        let usage = if let Some(usage_metadata) = response.usage_metadata {
            crate::llm::TokenUsage {
                prompt_tokens: usage_metadata.prompt_token_count,
                completion_tokens: usage_metadata.candidates_token_count,
                total_tokens: usage_metadata.total_token_count,
                estimated_cost: self.calculate_cost_from_metadata(&usage_metadata, model),
            }
        } else {
            // Fallback if no usage metadata
            crate::llm::TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                estimated_cost: 0.0,
            }
        };

        Ok(LLMResponse {
            id: format!("google-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![choice],
            usage,
            provider: LLMProviderType::Google,
            routing_info: RoutingInfo {
                selected_provider: LLMProviderType::Google,
                routing_strategy: RoutingStrategy::ModelSpecific("google".to_string()),
                latency_ms: 0,
                retry_count: 0,
                fallback_used: false,
                provider_used: LLMProviderType::Google,
                total_latency_ms: 0,
                provider_latency_ms: 0,
            },
        })
    }

    /// Calculate cost for Google usage from metadata
    fn calculate_cost_from_metadata(&self, usage: &GoogleUsageMetadata, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.prompt_token_count as f64 * input_cost) + (usage.candidates_token_count as f64 * output_cost)
        } else {
            // Fallback to Gemini Pro pricing if model not found
            (usage.prompt_token_count as f64 * 0.0000005) + (usage.candidates_token_count as f64 * 0.0000015)
        }
    }

    /// Handle error responses from Google
    fn handle_error_response(&self, status_code: u16, error_text: &str) -> LLMError {
        // Try to parse as Google error format
        if let Ok(google_error) = serde_json::from_str::<GoogleError>(error_text) {
            match status_code {
                401 => LLMError::AuthenticationFailed(google_error.error.message),
                429 => LLMError::RateLimitExceeded(google_error.error.message),
                400 => LLMError::InvalidRequest(google_error.error.message),
                _ => LLMError::Internal(format!(
                    "Google API error ({}): {}",
                    status_code, google_error.error.message
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

    /// Build request URL for Google API
    fn build_request_url(&self, model: &str, api_key: &str) -> String {
        format!("{}/models/{}:generateContent?key={}", 
            self.config.base_url, model, api_key)
    }
}

/// Find the end position of a complete JSON object in the buffer
fn find_complete_json_object(buffer: &str) -> Option<usize> {
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    for (i, ch) in buffer.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => {
                brace_count -= 1;
                if brace_count == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    
    None
}

/// Parse a Google JSON chunk into a StreamingChunk
fn parse_google_json_chunk(json_str: &str, request_id: &str, model: &str) -> LLMResult<Option<StreamingChunk>> {
    use super::types::GoogleResponse;
    
    let google_response: GoogleResponse = serde_json::from_str(json_str)
        .map_err(|e| LLMError::Parse(format!("Failed to parse Google JSON chunk: {}", e)))?;
    
    if let Some(candidate) = google_response.candidates.first() {
        if let Some(content) = &candidate.content {
            let text = content.parts
                .iter()
                .map(|part| part.text.clone())
                .collect::<Vec<String>>()
                .join("");
            
            if !text.is_empty() || candidate.finish_reason.is_some() {
                Ok(Some(StreamingChunk {
                    id: request_id.to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: model.to_string(),
                    choices: vec![StreamingChoice {
                        index: candidate.index.unwrap_or(0),
                        delta: ChatMessage {
                            role: MessageRole::Assistant,
                            content: text,
                            name: None,
                            function_call: None,
                        },
                        finish_reason: candidate.finish_reason.clone(),
                    }],
                    provider: LLMProviderType::Google,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[async_trait]
impl LLMProviderClient for GoogleClient {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse> {
        // Update config with provided API key if different
        let mut client_config = self.config.clone();
        if api_key != self.config.api_key {
            client_config.api_key = api_key.to_string();
        }
        let temp_client = GoogleClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let google_request = temp_client.convert_request(request)?;

        let request_url = temp_client.build_request_url(&request.model, api_key);
        
        debug!("Google API Request: URL={}, Model={}", request_url, request.model);
        debug!("API key: {}...", &api_key[..8.min(api_key.len())]);

        let response = temp_client.client
            .post(&request_url)
            .headers(headers)
            .json(&google_request)
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

            error!("Google API Error: {} - {}", status, error_text);
            return Err(temp_client.handle_error_response(status.as_u16(), &error_text));
        }

        // Debug: Log the raw response to understand the structure
        let response_text = response
            .text()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;
        
        debug!("Google API Raw Response: {}", response_text);
        
        let google_response: GoogleResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Google Response Deserialization Error: {}", e);
                debug!("Response was: {}", response_text);
                LLMError::Serialization(format!("Failed to parse Google response: {}", e))
            })?;

        debug!("Parsed Google Response: {} candidates", google_response.candidates.len());
        for (i, candidate) in google_response.candidates.iter().enumerate() {
            debug!("Candidate {}: has_content={}, finish_reason={:?}", 
                     i, candidate.content.is_some(), candidate.finish_reason);
            if let Some(content) = &candidate.content {
                debug!("Content parts count: {}", content.parts.len());
                for (j, part) in content.parts.iter().enumerate() {
                    debug!("Part {}: '{}'", j, part.text);
                }
            }
        }

        temp_client.convert_response(google_response, &request.model)
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
        let temp_client = GoogleClient::new(client_config);

        let google_request = temp_client.convert_request(&request)?;
        
        // Google uses streaming with the streamGenerateContent endpoint
        let request_url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            temp_client.config.base_url,
            &request.model,
            &api_key
        );
        


        let response = temp_client.client
            .post(&request_url)
            .header("Content-Type", "application/json")
            .json(&google_request)
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



        // Google uses JSON array streaming format, not SSE
        let request_id = request.id.to_string();
        let model = request.model;

        let stream = response.bytes_stream();
        let buffer = String::new();
        let chunk_index = 0;
        


        let google_stream = futures::stream::unfold(
            (stream, buffer, chunk_index, request_id, model),
            |(mut stream, mut buffer, mut chunk_index, request_id, model)| async move {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            let chunk_str = String::from_utf8_lossy(&bytes);

                            
                            buffer.push_str(&chunk_str);
                            chunk_index += 1;
                            
                            // Try to parse complete JSON objects from buffer
                            while let Some(json_end) = find_complete_json_object(&buffer) {
                                let json_str = buffer[..json_end].trim().to_string();
                                buffer = buffer[json_end..].to_string();
                            
                                if !json_str.is_empty() {
                                    // Remove leading comma or bracket if present
                                    let clean_json = json_str.trim_start_matches(',').trim_start_matches('[').trim();
                                
                                    if !clean_json.is_empty() {
                                        match parse_google_json_chunk(&clean_json, &request_id, &model) {
                                            Ok(Some(chunk)) => {

                                                return Some((Ok(chunk), (stream, buffer, chunk_index, request_id, model)));
                                            }
                                            Ok(None) => {

                                            }
                                            Err(e) => {

                                                return Some((Err(e), (stream, buffer, chunk_index, request_id, model)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_e) => {

                            return Some((Err(LLMError::Network(_e.to_string())), (stream, buffer, chunk_index, request_id, model)));
                        }
                    }
                }
                
                // Process any remaining buffer content when stream ends
                if !buffer.trim().is_empty() {
                    let clean_json = buffer.trim().trim_end_matches(']').trim();
                    if !clean_json.is_empty() {
                        match parse_google_json_chunk(clean_json, &request_id, &model) {
                            Ok(Some(chunk)) => {

                                return Some((Ok(chunk), (stream, String::new(), chunk_index, request_id, model)));
                            }
                            Ok(None) => {

                            }
                            Err(e) => {

                            }
                        }
                    }
                }
                

                None
            }
        );


        Ok(Box::new(Box::pin(google_stream)))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Google
    }

    async fn health_check(&self, api_key: &str) -> LLMResult<bool> {
        let mut client_config = self.config.clone();
        client_config.api_key = api_key.to_string();
        let temp_client = GoogleClient::new(client_config);

        let headers = temp_client.build_headers()?;
        let request_url = format!("{}/models?key={}", temp_client.config.base_url, api_key);

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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn embeddings(&self, _request: &EmbeddingsRequest, _api_key: &str) -> LLMResult<EmbeddingsResponse> {
        // TODO: Implement Google embeddings support
        Err(LLMError::Provider("Embeddings not yet implemented for Google provider".to_string()))
    }
}

impl CostCalculator for GoogleClient {
    fn calculate_cost(&self, usage: &crate::llm::TokenUsage, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (usage.prompt_tokens as f64 * input_cost) + (usage.completion_tokens as f64 * output_cost)
        } else {
            // Fallback to Gemini Pro pricing
            (usage.prompt_tokens as f64 * 0.0000005) + (usage.completion_tokens as f64 * 0.0000015)
        }
    }

    fn estimate_cost(&self, input_tokens: u32, estimated_output_tokens: u32, model: &str) -> f64 {
        if let Some((input_cost, output_cost)) = super::config::get_model_cost_info(model) {
            (input_tokens as f64 * input_cost) + (estimated_output_tokens as f64 * output_cost)
        } else {
            // Fallback to Gemini Pro pricing
            (input_tokens as f64 * 0.0000005) + (estimated_output_tokens as f64 * 0.0000015)
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
            // Fallback to Gemini Pro pricing
            let input_cost_total = usage.prompt_tokens as f64 * 0.0000005;
            let output_cost_total = usage.completion_tokens as f64 * 0.0000015;
            
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
    fn test_google_client_creation() {
        let config = GoogleConfig::default();
        let client = GoogleClient::new(config);
        assert_eq!(client.provider_type(), LLMProviderType::Google);
    }

    #[test]
    fn test_convert_request() {
        let client = GoogleClient::with_api_key("test-key".to_string());
        let request = crate::llm::LLMRequest {
            id: uuid::Uuid::new_v4(),
            model: "gemini-pro".to_string(),
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
                }
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

        let google_request = client.convert_request(&request).unwrap();
        assert!(!google_request.contents.is_empty());
        assert!(google_request.generation_config.is_some());
        
        let gen_config = google_request.generation_config.unwrap();
        assert_eq!(gen_config.temperature, Some(0.7));
        assert_eq!(gen_config.max_output_tokens, Some(100));
    }

    #[test]
    fn test_model_support() {
        let client = GoogleClient::with_api_key("test-key".to_string());
        assert!(client.supports_model("gemini-pro"));
        assert!(client.supports_model("gemini-2.5-flash-preview-05-20"));
        assert!(client.supports_model("gemini-1.5-pro"));
        assert!(!client.supports_model("gpt-4"));
        assert!(!client.supports_model("claude-3"));
    }

    #[test]
    fn test_build_request_url() {
        let client = GoogleClient::with_api_key("test-key".to_string());
        let url = client.build_request_url("gemini-pro", "test-api-key");
        assert!(url.contains("gemini-pro"));
        assert!(url.contains("generateContent"));
        assert!(url.contains("test-api-key"));
    }

    #[test]
    fn test_is_gemini_model() {
        use crate::llm::providers::google::is_gemini_model;
        
        assert!(is_gemini_model("gemini-pro"));
        assert!(is_gemini_model("gemini-1.5-flash"));
        assert!(is_gemini_model("gemini-2.5-flash-preview-05-20"));
        assert!(!is_gemini_model("gpt-4"));
        assert!(!is_gemini_model("claude-3"));
    }
}