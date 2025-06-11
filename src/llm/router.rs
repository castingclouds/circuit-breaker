//! LLM Router with Modular Provider System
//!
//! This module implements a router that uses the new modular provider architecture
//! with support for multiple providers and proper API key management.

use super::traits::LLMProviderClient;
use super::providers;
use super::*;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{warn, info, debug, error};


/// Provider health status tracking
#[derive(Debug, Clone)]
pub struct ProviderHealthStatus {
    pub is_healthy: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

impl Default for ProviderHealthStatus {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_check: chrono::Utc::now(),
            consecutive_failures: 0,
            last_error: None,
        }
    }
}

/// LLM Router configuration
#[derive(Debug, Clone)]
pub struct LLMRouterConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub health_check_interval_seconds: u64,
    pub enable_cost_tracking: bool,
    pub enable_health_monitoring: bool,
}

impl Default for LLMRouterConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval_seconds: 300, // 5 minutes
            enable_cost_tracking: true,
            enable_health_monitoring: true,
        }
    }
}

/// Main LLM Router using modular provider system
pub struct LLMRouter {
    config: LLMRouterConfig,
    providers: HashMap<LLMProviderType, Box<dyn LLMProviderClient>>,
    health_status: Arc<RwLock<HashMap<LLMProviderType, ProviderHealthStatus>>>,
    configured_api_keys: HashMap<LLMProviderType, String>,
}

impl LLMRouter {
    /// Create a new LLM router with default configuration
    pub async fn new() -> Result<Self, LLMError> {
        Self::new_with_keys(None, None, None, None).await
    }

    /// Create a new LLM router for testing (allows empty providers)
    pub async fn new_for_testing() -> Result<Self, LLMError> {
        let config = LLMRouterConfig::default();
        let providers = HashMap::new();
        let health_status = HashMap::new();
        let configured_api_keys = HashMap::new();

        Ok(Self {
            config,
            providers,
            health_status: Arc::new(RwLock::new(health_status)),
            configured_api_keys,
        })
    }

    /// Create a new LLM router with provided API keys
    pub async fn new_with_keys(
        openai_key: Option<String>,
        anthropic_key: Option<String>, 
        google_key: Option<String>,
        ollama_base_url: Option<String>,
    ) -> Result<Self, LLMError> {
        let config = LLMRouterConfig::default();
        let mut providers = HashMap::new();
        let mut health_status = HashMap::new();
        let mut configured_api_keys = HashMap::new();

        // Initialize OpenAI provider if key is available
        if let Some(key) = openai_key.or_else(|| std::env::var("OPENAI_API_KEY").ok()) {
            let client = providers::openai::create_client(key.clone(), None);
            providers.insert(LLMProviderType::OpenAI, Box::new(client) as Box<dyn LLMProviderClient>);
            health_status.insert(LLMProviderType::OpenAI, ProviderHealthStatus::default());
            configured_api_keys.insert(LLMProviderType::OpenAI, key);
            info!("✅ OpenAI provider initialized");
        }

        // Initialize Anthropic provider if key is available
        if let Some(key) = anthropic_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()) {
            let client = providers::anthropic::create_client(key.clone(), None);
            providers.insert(LLMProviderType::Anthropic, Box::new(client) as Box<dyn LLMProviderClient>);
            health_status.insert(LLMProviderType::Anthropic, ProviderHealthStatus::default());
            configured_api_keys.insert(LLMProviderType::Anthropic, key);
            info!("✅ Anthropic provider initialized");
        }

        // Initialize Google provider if key is available
        if let Some(key) = google_key.or_else(|| std::env::var("GOOGLE_API_KEY").ok()) {
            let client = providers::google::create_client(key.clone(), None);
            providers.insert(LLMProviderType::Google, Box::new(client) as Box<dyn LLMProviderClient>);
            health_status.insert(LLMProviderType::Google, ProviderHealthStatus::default());
            configured_api_keys.insert(LLMProviderType::Google, key);
            info!("✅ Google provider initialized");
        }

        // Initialize Ollama provider if base URL is available
        let ollama_url = ollama_base_url.or_else(|| std::env::var("OLLAMA_BASE_URL").ok()).unwrap_or_else(|| "http://localhost:11434".to_string());
        
        // Check if Ollama is available by trying to connect
        if providers::ollama::check_availability(&ollama_url).await {
            let client = providers::ollama::create_client(ollama_url.clone());
            providers.insert(LLMProviderType::Ollama, Box::new(client) as Box<dyn LLMProviderClient>);
            health_status.insert(LLMProviderType::Ollama, ProviderHealthStatus::default());
            configured_api_keys.insert(LLMProviderType::Ollama, ollama_url);
            info!("✅ Ollama provider initialized");
        } else {
            warn!("⚠️  Ollama not available at {} - skipping initialization", ollama_url);
        }

        // Initialize vLLM provider if base URL is available
        let vllm_url = std::env::var("VLLM_BASE_URL").ok().unwrap_or_else(|| "http://localhost:8000".to_string());
        
        // Check if vLLM is available by trying to connect
        if providers::vllm::check_availability(&vllm_url).await {
            let client = providers::vllm::create_client(vllm_url.clone());
            providers.insert(LLMProviderType::VLLM, Box::new(client) as Box<dyn LLMProviderClient>);
            health_status.insert(LLMProviderType::VLLM, ProviderHealthStatus::default());
            configured_api_keys.insert(LLMProviderType::VLLM, vllm_url);
            info!("✅ vLLM provider initialized");
        } else {
            warn!("⚠️  vLLM not available at {} - skipping initialization", vllm_url);
        }

        if providers.is_empty() {
            warn!("No providers configured with valid API keys - router will have limited functionality");
        }

        Ok(Self {
            config,
            providers,
            health_status: Arc::new(RwLock::new(health_status)),
            configured_api_keys,
        })
    }

    /// Route a chat completion request to the appropriate provider
    pub async fn chat_completion(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        // Resolve virtual model to actual model
        let resolved_model = self.resolve_virtual_model(&request.model);
        let provider_type = self.determine_provider_for_model(&resolved_model);
        
        debug!("Router: Model '{}' -> Resolved '{}' -> Provider '{}'", request.model, resolved_model, provider_type);
        
        let provider_client = self.providers.get(&provider_type)
            .ok_or_else(|| LLMError::Internal(format!("Provider {} not available", provider_type)))?;

        let api_key = self.get_api_key(&provider_type).await?;

        // Create modified request with resolved model name
        let mut resolved_request = request.clone();
        resolved_request.model = resolved_model.clone();

        let max_retries = self.config.max_retries;
        let mut retry_count = 0;

        while retry_count <= max_retries {
            match provider_client.chat_completion(&resolved_request, &api_key).await {
                Ok(mut response) => {
                    // Update routing info
                    response.routing_info.latency_ms = 0; // TODO: Measure actual latency
                    response.routing_info.retry_count = retry_count;
                    response.routing_info.selected_provider = provider_type.clone();
                    response.routing_info.routing_strategy = 
                        RoutingStrategy::ModelSpecific(provider_type.to_string());
                    response.routing_info.fallback_used = retry_count > 0;

                    // Update health status on success
                    self.update_health_success(&provider_type).await;

                    return Ok(response);
                }
                Err(e) => {
                    warn!("Request failed for provider {}: {}", provider_type, e);
                    retry_count += 1;

                    // Update health status on failure
                    self.update_health_failure(&provider_type, &e).await;

                    if retry_count <= max_retries {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            self.config.retry_delay_ms * retry_count as u64,
                        )).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(LLMError::Internal("All retry attempts failed".to_string()))
    }

    /// Route a streaming chat completion request
    pub async fn stream_chat_completion(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        let provider = self.determine_provider_for_model(&request.model);
        let api_key = self.get_api_key(&provider).await?;
        
        if let Some(client) = self.providers.get(&provider) {
            let stream_result = client.chat_completion_stream(request.clone(), api_key).await;
            match stream_result {
                Ok(stream) => {
                    Ok(Box::new(Box::pin(stream)))
                }
                Err(e) => {
                    error!("Router: provider returned error: {}", e);
                    Err(e)
                }
            }
        } else {
            // For unsupported providers, fall back to mock streaming
            let response = self.chat_completion(request).await?;
            
            let chunk = StreamingChunk {
                id: response.id,
                object: "chat.completion.chunk".to_string(),
                created: response.created,
                model: response.model,
                choices: response.choices.into_iter().map(|choice| StreamingChoice {
                    index: choice.index,
                    delta: choice.message,
                    finish_reason: choice.finish_reason,
                }).collect(),
                provider: response.provider,
            };
            
            let stream = futures::stream::once(async move { Ok(chunk) });
            Ok(Box::new(Box::pin(stream)))
        }
    }

    /// Resolve virtual model name to actual model name
    pub fn resolve_virtual_model(&self, model: &str) -> String {
        match model {
            "auto" => "claude-3-haiku-20240307".to_string(), // Default to fast, cost-effective model
            "cb:smart-chat" => "claude-3-sonnet-20240229".to_string(), // Balanced model for chat
            "cb:cost-optimal" => "claude-3-haiku-20240307".to_string(), // Cheapest option
            "cb:fastest" => "o4-mini-2025-04-16".to_string(), // Fastest response
            "cb:coding" => "claude-3-5-sonnet-20240620".to_string(), // Best for coding
            "cb:analysis" => "claude-3-5-sonnet-20240620".to_string(), // Best for analysis
            "cb:creative" => "claude-3-opus-20240229".to_string(), // Most creative
            _ => model.to_string(), // Return as-is if not a virtual model
        }
    }

    /// Determine which provider to use based on model name
    pub fn determine_provider_for_model(&self, model: &str) -> LLMProviderType {
        if model.starts_with("gpt-") || model.starts_with("o4-") {
            LLMProviderType::OpenAI
        } else if model.starts_with("gemini-") || model == "gemini-pro" {
            LLMProviderType::Google
        } else if model.starts_with("claude-") {
            LLMProviderType::Anthropic
        } else {
            // Check if any provider supports this model
            for (provider_type, client) in &self.providers {
                if client.supports_model(model) {
                    return provider_type.clone();
                }
            }
            
            // Default to the first available provider
            self.providers.keys().next().cloned()
                .unwrap_or(LLMProviderType::OpenAI)
        }
    }

    /// Get API key for provider
    async fn get_api_key(&self, provider_type: &LLMProviderType) -> LLMResult<String> {
        debug!("Getting API key for provider: {}", provider_type);
        debug!("Configured keys available: {:?}", self.configured_api_keys.keys().collect::<Vec<_>>());
        
        if let Some(key) = self.configured_api_keys.get(provider_type) {
            debug!("Found configured key for {}: {}...", provider_type, &key[..8.min(key.len())]);
            return Ok(key.clone());
        }

        debug!("No configured key found for {}, checking environment...", provider_type);
        
        // Fallback to environment variables if not configured
        match provider_type {
            LLMProviderType::OpenAI => std::env::var("OPENAI_API_KEY").map_err(|_| {
                LLMError::AuthenticationFailed(
                    "OPENAI_API_KEY not configured in server or environment".to_string(),
                )
            }),
            LLMProviderType::Anthropic => std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
                LLMError::AuthenticationFailed(
                    "ANTHROPIC_API_KEY not configured in server or environment".to_string(),
                )
            }),
            LLMProviderType::Google => std::env::var("GOOGLE_API_KEY").map_err(|_| {
                LLMError::AuthenticationFailed(
                    "GOOGLE_API_KEY not configured in server or environment".to_string(),
                )
            }),
            _ => Err(LLMError::AuthenticationFailed(
                format!("API key not configured for provider: {}", provider_type),
            )),
        }
    }

    /// Update health status on successful request
    async fn update_health_success(&self, provider_type: &LLMProviderType) {
        let mut health_map = self.health_status.write().await;
        if let Some(status) = health_map.get_mut(provider_type) {
            status.is_healthy = true;
            status.consecutive_failures = 0;
            status.last_check = chrono::Utc::now();
            status.last_error = None;
        }
    }

    /// Update health status on failed request
    async fn update_health_failure(&self, provider_type: &LLMProviderType, error: &LLMError) {
        let mut health_map = self.health_status.write().await;
        if let Some(status) = health_map.get_mut(provider_type) {
            status.consecutive_failures += 1;
            status.last_check = chrono::Utc::now();
            status.last_error = Some(error.to_string());
            
            // Mark as unhealthy after 3 consecutive failures
            if status.consecutive_failures >= 3 {
                status.is_healthy = false;
                warn!("Provider {} marked as unhealthy after {} failures", provider_type, status.consecutive_failures);
            }
        }
    }

    /// Get health status for a provider
    pub async fn get_provider_health(&self, provider_type: &LLMProviderType) -> Option<ProviderHealthStatus> {
        let health_map = self.health_status.read().await;
        health_map.get(provider_type).cloned()
    }

    /// Get all available providers
    pub fn get_available_providers(&self) -> Vec<LLMProviderType> {
        self.providers.keys().cloned().collect()
    }

    /// Check if a provider is available
    pub fn is_provider_available(&self, provider_type: &LLMProviderType) -> bool {
        self.providers.contains_key(provider_type)
    }

    /// Get provider client for direct access (for advanced use cases)
    pub fn get_provider_client(&self, provider_type: &LLMProviderType) -> Option<&dyn LLMProviderClient> {
        self.providers.get(provider_type).map(|client| client.as_ref())
    }

    /// Get providers (for GraphQL compatibility)
    pub async fn get_providers(&self) -> Vec<LLMProvider> {
        let mut providers = Vec::new();
        for (provider_type, client) in &self.providers {
            let models = match provider_type {
                LLMProviderType::Ollama => {
                    // For Ollama, fetch actual models from the instance
                    if let Some(ollama_client) = client.as_any().downcast_ref::<crate::llm::providers::ollama::OllamaClient>() {
                        ollama_client.get_available_models_async().await
                    } else {
                        client.get_available_models()
                    }
                },
                LLMProviderType::VLLM => {
                    // For vLLM, fetch actual models from the server
                    if let Some(vllm_client) = client.as_any().downcast_ref::<crate::llm::providers::vllm::VLLMClient>() {
                        vllm_client.get_available_models_async().await
                    } else {
                        client.get_available_models()
                    }
                },
                _ => client.get_available_models(),
            };
            let llm_models: Vec<LLMModel> = models.into_iter().map(|model| LLMModel {
                id: model.id,
                name: model.name,
                provider_id: uuid::Uuid::new_v4(),
                max_tokens: model.max_output_tokens,
                context_window: model.context_window,
                cost_per_input_token: model.cost_per_input_token,
                cost_per_output_token: model.cost_per_output_token,
                supports_streaming: model.supports_streaming,
                supports_function_calling: model.supports_function_calling,
                capabilities: model.capabilities.into_iter().map(|cap| match cap {
                    crate::llm::traits::ModelCapability::TextGeneration => ModelCapability::TextGeneration,
                    crate::llm::traits::ModelCapability::CodeGeneration => ModelCapability::CodeGeneration,
                    crate::llm::traits::ModelCapability::ConversationalAI => ModelCapability::TextAnalysis,
                    crate::llm::traits::ModelCapability::FunctionCalling => ModelCapability::FunctionCalling,
                    crate::llm::traits::ModelCapability::Translation => ModelCapability::Translation,
                    crate::llm::traits::ModelCapability::Summarization => ModelCapability::Summarization,
                    crate::llm::traits::ModelCapability::ReasoningChain => ModelCapability::Reasoning,
                    _ => ModelCapability::TextGeneration,
                }).collect(),
            }).collect();

            providers.push(LLMProvider {
                id: uuid::Uuid::new_v4(),
                provider_type: provider_type.clone(),
                name: format!("{:?}", provider_type),
                base_url: match provider_type {
                    LLMProviderType::OpenAI => "https://api.openai.com/v1".to_string(),
                    LLMProviderType::Anthropic => "https://api.anthropic.com".to_string(),
                    LLMProviderType::Google => "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    LLMProviderType::Ollama => self.configured_api_keys.get(provider_type).cloned().unwrap_or_else(|| "http://localhost:11434".to_string()),
                    _ => "".to_string(),
                },
                api_key_id: Some(format!("{}_key", provider_type.to_string())),
                models: llm_models,
                rate_limits: RateLimits::default(),
                health_status: super::ProviderHealthStatus::default(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });
        }
        providers
    }

    /// Get health status for all providers (for GraphQL compatibility)
    pub async fn get_health_status(&self) -> HashMap<LLMProviderType, ProviderHealthStatus> {
        self.health_status.read().await.clone()
    }

    /// Smart chat completion (for API handler compatibility)
    pub async fn smart_chat_completion(
        &self, 
        request: LLMRequest, 
        _config: Option<crate::api::types::CircuitBreakerConfig>
    ) -> LLMResult<LLMResponse> {
        // For now, just use regular chat completion
        self.chat_completion(request).await
    }

    /// Smart streaming chat completion (for API handler compatibility)
    pub async fn smart_chat_completion_stream(
        &self,
        request: LLMRequest,
        _config: Option<crate::api::types::CircuitBreakerConfig>
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        // For now, just use regular streaming
        self.stream_chat_completion(request).await
    }

    /// Generate embeddings using the appropriate provider
    pub async fn embeddings(&self, request: &crate::llm::EmbeddingsRequest, api_key: &str) -> LLMResult<crate::llm::EmbeddingsResponse> {
        let provider_type = self.determine_provider_for_model(&request.model);
        
        if let Some(client) = self.providers.get(&provider_type) {
            client.embeddings(request, api_key).await
        } else {
            Err(LLMError::Provider(format!("No provider available for model: {}", request.model)))
        }
    }

    /// Run health checks on all providers
    pub async fn run_health_checks(&self) {
        if !self.config.enable_health_monitoring {
            return;
        }

        for (provider_type, client) in &self.providers {
            if let Ok(api_key) = self.get_api_key(provider_type).await {
                match client.health_check(&api_key).await {
                    Ok(is_healthy) => {
                        if is_healthy {
                            self.update_health_success(provider_type).await;
                        } else {
                            let error = LLMError::Internal("Health check failed".to_string());
                            self.update_health_failure(provider_type, &error).await;
                        }
                    }
                    Err(error) => {
                        self.update_health_failure(provider_type, &error).await;
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for LLMRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LLMRouter with {} providers: {:?}", 
            self.providers.len(), 
            self.providers.keys().collect::<Vec<_>>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_creation_without_keys() {
        // Should succeed without any API keys now (warns instead of fails)
        let result = LLMRouter::new_with_keys(None, None, None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_provider_determination() {
        // Mock router with OpenAI key
        if let Ok(router) = LLMRouter::new_with_keys(Some("test-key".to_string()), None, None, None).await {
            assert_eq!(router.determine_provider_for_model("gpt-4"), LLMProviderType::OpenAI);
            assert_eq!(router.determine_provider_for_model("o4-mini-2025-04-16"), LLMProviderType::OpenAI);
            assert_eq!(router.determine_provider_for_model("claude-3"), LLMProviderType::Anthropic); // Correctly determines Anthropic
            assert_eq!(router.determine_provider_for_model("unknown-model"), LLMProviderType::OpenAI); // Falls back to available provider
        }
    }

    #[test]
    fn test_router_display() {
        // Test that the router formats correctly
        let router = LLMRouter {
            config: LLMRouterConfig::default(),
            providers: HashMap::new(),
            health_status: Arc::new(RwLock::new(HashMap::new())),
            configured_api_keys: HashMap::new(),
        };
        
        let display = format!("{}", router);
        assert!(display.contains("LLMRouter"));
        assert!(display.contains("0 providers"));
    }
}