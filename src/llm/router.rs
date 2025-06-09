//! Simplified LLM Router Implementation
//!
//! This module implements a simplified router focused on Anthropic provider
//! with real API integration and cost tracking.

use super::providers::{LLMProviderClient, create_provider_client};
use super::*;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{error, warn, info};
use crate::api::types::{CircuitBreakerConfig, is_virtual_model};

/// Simplified LLM Router for Anthropic
pub struct LLMRouter {
    _config: LLMRouterConfig,
    providers: HashMap<LLMProviderType, Box<dyn LLMProviderClient>>,
    provider_configs: HashMap<LLMProviderType, LLMProvider>,
    health_status: Arc<RwLock<HashMap<LLMProviderType, ProviderHealthStatus>>>,
    _api_keys: Arc<SimpleApiKeyStorage>,
    configured_api_keys: HashMap<LLMProviderType, String>,
}

impl LLMRouter {
    /// Create a new LLM router with simplified setup
    pub async fn new() -> Result<Self, LLMError> {
        Self::new_with_keys(None, None, None).await
    }

    /// Create a new LLM router with provided API keys
    pub async fn new_with_keys(
        openai_key: Option<String>,
        anthropic_key: Option<String>, 
        google_key: Option<String>,
    ) -> Result<Self, LLMError> {
        let mut providers = HashMap::new();
        let mut configs = HashMap::new();
        let mut health_status = HashMap::new();

        // Default OpenAI provider configuration
        let openai_config = LLMProvider {
            id: uuid::Uuid::new_v4(),
            provider_type: LLMProviderType::OpenAI,
            name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_id: Some("openai_key".to_string()),
            models: vec![
                LLMModel {
                    id: "gpt-3.5-turbo".to_string(),
                    name: "GPT-3.5 Turbo".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 4096,
                    context_window: 16385,
                    cost_per_input_token: 0.000001,
                    cost_per_output_token: 0.000002,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::FunctionCalling,
                    ],
                },
                LLMModel {
                    id: "gpt-3.5-turbo".to_string(),
                    name: "GPT-3.5 Turbo".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 4096,
                    context_window: 16385,
                    cost_per_input_token: 0.000001,
                    cost_per_output_token: 0.000002,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::FunctionCalling,
                    ],
                },
                LLMModel {
                    id: "o4-mini-2025-04-16".to_string(),
                    name: "OpenAI o4 Mini".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 16384,
                    context_window: 128000,
                    cost_per_input_token: 0.000001,
                    cost_per_output_token: 0.000002,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                        ModelCapability::FunctionCalling,
                    ],
                },
            ],
            rate_limits: RateLimits::default(),
            health_status: ProviderHealthStatus::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Default Google provider configuration
        let google_config = LLMProvider {
            id: uuid::Uuid::new_v4(),
            provider_type: LLMProviderType::Google,
            name: "Google Gemini".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            api_key_id: Some("google_key".to_string()),
            models: vec![
                LLMModel {
                    id: "gemini-pro".to_string(),
                    name: "Gemini Pro".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 8192,
                    context_window: 32768,
                    cost_per_input_token: 0.0000005,
                    cost_per_output_token: 0.0000015,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                        ModelCapability::FunctionCalling,
                    ],
                },
                LLMModel {
                    id: "gemini-2.5-flash-preview-05-20".to_string(),
                    name: "Gemini 2.5 Flash Preview".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 8192,
                    context_window: 1048576,
                    cost_per_input_token: 0.000000075,
                    cost_per_output_token: 0.0000003,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                        ModelCapability::FunctionCalling,
                    ],
                },
            ],
            rate_limits: RateLimits::default(),
            health_status: ProviderHealthStatus::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Default Anthropic provider configuration
        let anthropic_config = LLMProvider {
            id: uuid::Uuid::new_v4(),
            provider_type: LLMProviderType::Anthropic,
            name: "Anthropic Claude".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key_id: Some("anthropic_key".to_string()),
            models: vec![
                LLMModel {
                    id: "claude-3-haiku-20240307".to_string(),
                    name: "Claude 3 Haiku".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 4096,
                    context_window: 200000,
                    cost_per_input_token: 0.00000025,
                    cost_per_output_token: 0.00000125,
                    supports_streaming: true,
                    supports_function_calling: false,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                    ],
                },
                LLMModel {
                    id: "claude-3-sonnet-20240229".to_string(),
                    name: "Claude 3 Sonnet".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 4096,
                    context_window: 200000,
                    cost_per_input_token: 0.000003,
                    cost_per_output_token: 0.000015,
                    supports_streaming: true,
                    supports_function_calling: false,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                    ],
                },
                LLMModel {
                    id: "claude-sonnet-4-20250514".to_string(),
                    name: "Claude 4".to_string(),
                    provider_id: uuid::Uuid::new_v4(),
                    max_tokens: 8192,
                    context_window: 200000,
                    cost_per_input_token: 0.000003,
                    cost_per_output_token: 0.000015,
                    supports_streaming: true,
                    supports_function_calling: true,
                    capabilities: vec![
                        ModelCapability::TextGeneration,
                        ModelCapability::TextAnalysis,
                        ModelCapability::CodeGeneration,
                        ModelCapability::Reasoning,
                    ],
                },
            ],
            rate_limits: RateLimits::default(),
            health_status: ProviderHealthStatus::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Create provider clients
        let openai_client = create_provider_client(
            &openai_config.provider_type,
            Some(openai_config.base_url.clone()),
        );

        let google_client = create_provider_client(
            &google_config.provider_type,
            Some(google_config.base_url.clone()),
        );

        let anthropic_client = create_provider_client(
            &anthropic_config.provider_type,
            Some(anthropic_config.base_url.clone()),
        );

        providers.insert(LLMProviderType::OpenAI, openai_client);
        providers.insert(LLMProviderType::Google, google_client);
        providers.insert(LLMProviderType::Anthropic, anthropic_client);

        configs.insert(LLMProviderType::OpenAI, openai_config);
        configs.insert(LLMProviderType::Google, google_config);
        configs.insert(LLMProviderType::Anthropic, anthropic_config);

        health_status.insert(
            LLMProviderType::OpenAI,
            ProviderHealthStatus::default(),
        );
        health_status.insert(
            LLMProviderType::Google,
            ProviderHealthStatus::default(),
        );
        health_status.insert(
            LLMProviderType::Anthropic,
            ProviderHealthStatus::default(),
        );

        // Store configured API keys
        let mut configured_api_keys = HashMap::new();
        if let Some(key) = openai_key.or_else(|| std::env::var("OPENAI_API_KEY").ok()) {
            configured_api_keys.insert(LLMProviderType::OpenAI, key);
        }
        if let Some(key) = anthropic_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()) {
            configured_api_keys.insert(LLMProviderType::Anthropic, key);
        }
        if let Some(key) = google_key.or_else(|| std::env::var("GOOGLE_API_KEY").ok()) {
            configured_api_keys.insert(LLMProviderType::Google, key);
        }

        let router = Self {
            _config: LLMRouterConfig::default(),
            providers,
            provider_configs: configs,
            health_status: Arc::new(RwLock::new(health_status)),
            _api_keys: Arc::new(SimpleApiKeyStorage::new()),
            configured_api_keys,
        };

        Ok(router)
    }

    /// Route a chat completion request
    pub async fn chat_completion(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        // Determine provider based on model name
        let provider_type = self.determine_provider_for_model(&request.model);
        
        eprintln!("🔍 Router: Model '{}' -> Provider '{}'", request.model, provider_type);

        let provider_client = self
            .providers
            .get(&provider_type)
            .ok_or_else(|| LLMError::ProviderNotFound(provider_type.to_string()))?;

        // Get API key
        let api_key = self.get_api_key(&provider_type).await?;

        let mut retry_count = 0;
        let max_retries = 3;

        while retry_count <= max_retries {
            match provider_client.chat_completion(&request, &api_key).await {
                Ok(mut response) => {
                    // Update routing info
                    response.routing_info.retry_count = retry_count;
                    response.routing_info.routing_strategy =
                        RoutingStrategy::ModelSpecific("anthropic".to_string());

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
                            100 * retry_count as u64,
                        ))
                        .await;
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
        // Determine provider based on model name
        let provider_type = self.determine_provider_for_model(&request.model);
        
        eprintln!("🔍 Router Streaming: Model '{}' -> Provider '{}'", request.model, provider_type);
        
        // For now, use non-streaming and convert to stream to avoid lifetime issues
        // This is a temporary solution until we can properly implement streaming
        let response = self.chat_completion(request).await?;
        
        // Convert the single response into a stream
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

    /// Determine which provider to use based on model name
    pub fn determine_provider_for_model(&self, model: &str) -> LLMProviderType {
        if model.starts_with("gpt-") || model.starts_with("o4-") {
            LLMProviderType::OpenAI
        } else if model.starts_with("gemini-") || model == "gemini-pro" {
            LLMProviderType::Google
        } else if model.starts_with("claude-") {
            LLMProviderType::Anthropic
        } else {
            // Default to Anthropic for backward compatibility
            LLMProviderType::Anthropic
        }
    }

    /// Get API key for provider from configured keys or environment
    async fn get_api_key(&self, provider_type: &LLMProviderType) -> LLMResult<String> {
        eprintln!("🔍 Getting API key for provider: {}", provider_type);
        eprintln!("   Configured keys available: {:?}", self.configured_api_keys.keys().collect::<Vec<_>>());
        
        if let Some(key) = self.configured_api_keys.get(provider_type) {
            eprintln!("   ✅ Found configured key for {}: {}...", provider_type, &key[..8.min(key.len())]);
            return Ok(key.clone());
        }

        eprintln!("   ⚠️  No configured key found for {}, checking environment...", provider_type);
        
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
            _ => Err(LLMError::ProviderNotFound(provider_type.to_string())),
        }
    }

    /// Update health status on successful request
    async fn update_health_success(&self, provider_type: &LLMProviderType) {
        let mut health_status = self.health_status.write().await;
        if let Some(status) = health_status.get_mut(provider_type) {
            status.consecutive_failures = 0;
            status.last_error = None;
            status.error_rate = status.error_rate * 0.9; // Decay error rate
            status.is_healthy = true;
        }
    }

    /// Update health status on failed request
    async fn update_health_failure(&self, provider_type: &LLMProviderType, error: &LLMError) {
        let mut health_status = self.health_status.write().await;
        if let Some(status) = health_status.get_mut(provider_type) {
            status.consecutive_failures += 1;
            status.last_error = Some(error.to_string());
            status.error_rate = (status.error_rate * 0.9) + 0.1; // Increase error rate

            // Mark as unhealthy if too many consecutive failures
            if status.consecutive_failures >= 3 {
                status.is_healthy = false;
            }
        }
    }

    /// Get current health status for all providers
    pub async fn get_health_status(&self) -> HashMap<LLMProviderType, ProviderHealthStatus> {
        self.health_status.read().await.clone()
    }

    /// Get available providers
    pub async fn get_providers(&self) -> Vec<LLMProvider> {
        self.provider_configs.values().cloned().collect()
    }

    // ============================================================================
    // Smart Routing Extensions
    // These methods extend the existing router with intelligent routing capabilities
    // ============================================================================

    /// Smart routing method that extends existing chat_completion
    pub async fn smart_chat_completion(
        &self, 
        request: LLMRequest, 
        routing_config: Option<CircuitBreakerConfig>
    ) -> LLMResult<LLMResponse> {
        info!("Processing smart routing request for model: {}", request.model);
        
        // 1. Check if it's a virtual model or "auto"
        if is_virtual_model(&request.model) {
            return self.route_with_strategy(request, routing_config).await;
        }
        
        // 2. Check if it's a real model name (existing functionality)
        if self.has_model(&request.model).await {
            // Apply smart routing if config provided, otherwise use existing logic
            if let Some(config) = routing_config {
                return self.route_with_preferences(request, config).await;
            } else {
                // Use existing chat_completion method
                return self.chat_completion(request).await;
            }
        }
        
        // 3. Model not found, try smart selection
        self.auto_select_model(request, routing_config).await
    }

    /// Smart routing for streaming requests
    pub async fn smart_chat_completion_stream(
        &self,
        request: LLMRequest,
        routing_config: Option<CircuitBreakerConfig>
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>> {
        info!("Processing smart streaming request for model: {}", request.model);
        
        // For virtual models, resolve to actual model first
        let resolved_request = if is_virtual_model(&request.model) {
            let selected_model = self.resolve_virtual_model(&request.model, &routing_config).await?;
            LLMRequest {
                model: selected_model,
                ..request
            }
        } else {
            request
        };
        
        // Use existing streaming method with resolved model
        self.stream_chat_completion(resolved_request).await
    }
    
    /// Route request using strategy (extends existing routing)
    async fn route_with_strategy(
        &self, 
        mut request: LLMRequest, 
        config: Option<CircuitBreakerConfig>
    ) -> LLMResult<LLMResponse> {
        let default_strategy = "balanced".to_string();
        let strategy = config
            .as_ref()
            .and_then(|c| c.routing_strategy.as_ref())
            .unwrap_or(&default_strategy);
            
        info!("Routing with strategy: {}", strategy);
            
        // Select best provider based on strategy
        let selected_model = match strategy.as_str() {
            "cost_optimized" => self.select_cheapest_available().await?,
            "performance_first" => self.select_fastest_available().await?,
            "balanced" => self.select_balanced().await?,
            "reliability_first" => self.select_most_reliable().await?,
            _ => self.select_default().await?,
        };
        
        info!("Selected model: {} for strategy: {}", selected_model, strategy);
        
        // Update request with selected model
        request.model = selected_model;
        
        // Use existing chat_completion with selected model
        self.chat_completion(request).await
    }

    /// Route with user preferences (cost, latency constraints)
    async fn route_with_preferences(
        &self,
        mut request: LLMRequest,
        config: CircuitBreakerConfig
    ) -> LLMResult<LLMResponse> {
        info!("Routing with preferences: {:?}", config);
        
        // Try to use the specified model first if it meets constraints
        if self.model_meets_constraints(&request.model, &config).await {
            return self.chat_completion(request).await;
        }
        
        // If specified model doesn't meet constraints, find alternative
        let alternative_model = self.find_model_with_constraints(&config).await?;
        request.model = alternative_model;
        
        self.chat_completion(request).await
    }

    /// Auto-select best model when none specified
    async fn auto_select_model(
        &self,
        mut request: LLMRequest,
        _config: Option<CircuitBreakerConfig>
    ) -> LLMResult<LLMResponse> {
        info!("Auto-selecting model for request");
        
        let selected_model = if let Some(config) = &_config {
            self.find_model_with_constraints(config).await?
        } else {
            self.select_default().await?
        };
        
        request.model = selected_model;
        self.chat_completion(request).await
    }

    /// Resolve virtual model to actual model name
    async fn resolve_virtual_model(
        &self,
        virtual_model: &str,
        _config: &Option<CircuitBreakerConfig>
    ) -> LLMResult<String> {
        match virtual_model {
            "auto" => self.select_balanced().await,
            "cb:smart-chat" => self.select_balanced().await,
            "cb:cost-optimal" => self.select_cheapest_available().await,
            "cb:fastest" => self.select_fastest_available().await,
            "cb:coding" => self.select_best_for_task("coding").await,
            "cb:analysis" => self.select_best_for_task("analysis").await,
            "cb:creative" => self.select_best_for_task("creative").await,
            _ => {
                warn!("Unknown virtual model: {}, using default", virtual_model);
                self.select_default().await
            }
        }
    }
    
    /// Select cheapest available model
    async fn select_cheapest_available(&self) -> LLMResult<String> {
        let mut cheapest_model = None;
        let mut lowest_cost = f64::MAX;
        
        // First try to find the cheapest model among healthy providers
        for (provider_type, config) in &self.provider_configs {
            if self.is_provider_healthy(provider_type).await {
                for model in &config.models {
                    let total_cost = model.cost_per_input_token + model.cost_per_output_token;
                    if total_cost < lowest_cost {
                        lowest_cost = total_cost;
                        cheapest_model = Some(model.id.clone());
                    }
                }
            }
        }
        
        // If no healthy providers found, fall back to Anthropic as it's most likely to work
        if cheapest_model.is_none() {
            warn!("No healthy providers found, falling back to Anthropic");
            if let Some(anthropic_config) = self.provider_configs.get(&LLMProviderType::Anthropic) {
                if let Some(model) = anthropic_config.models.first() {
                    return Ok(model.id.clone());
                }
            }
        }
        
        cheapest_model.ok_or_else(|| {
            error!("No providers available");
            LLMError::Internal("No providers available".to_string())
        })
    }
    
    /// Select fastest available model (based on average latency)
    async fn select_fastest_available(&self) -> LLMResult<String> {
        let health_status = self.health_status.read().await;
        
        let mut fastest_model = None;
        let mut lowest_latency = u64::MAX;
        
        for (provider_type, config) in &self.provider_configs {
            if let Some(status) = health_status.get(provider_type) {
                if status.is_healthy && status.average_latency_ms < lowest_latency {
                    lowest_latency = status.average_latency_ms;
                    // Select the first model from the fastest provider
                    if let Some(model) = config.models.first() {
                        fastest_model = Some(model.id.clone());
                    }
                }
            }
        }
        
        fastest_model.ok_or_else(|| {
            error!("No healthy providers available for performance optimization");
            LLMError::Internal("No healthy providers available".to_string())
        })
    }
    
    /// Select balanced model (considers both cost and performance)
    async fn select_balanced(&self) -> LLMResult<String> {
        let health_status = self.health_status.read().await;
        
        let mut best_model = None;
        let mut best_score = f64::MIN;
        
        for (provider_type, config) in &self.provider_configs {
            if let Some(status) = health_status.get(provider_type) {
                if status.is_healthy {
                    for model in &config.models {
                        // Calculate balance score (lower cost and latency is better)
                        let cost_score = 1.0 / (model.cost_per_input_token + model.cost_per_output_token + 0.000001);
                        let latency_score = 1.0 / (status.average_latency_ms as f64 + 1.0);
                        let balance_score = (cost_score + latency_score) / 2.0;
                        
                        if balance_score > best_score {
                            best_score = balance_score;
                            best_model = Some(model.id.clone());
                        }
                    }
                }
            }
        }
        
        // If no healthy providers found, fall back to Anthropic
        if best_model.is_none() {
            warn!("No healthy providers found for balanced selection, falling back to Anthropic");
            if let Some(anthropic_config) = self.provider_configs.get(&LLMProviderType::Anthropic) {
                if let Some(model) = anthropic_config.models.first() {
                    return Ok(model.id.clone());
                }
            }
        }
        
        best_model.ok_or_else(|| {
            error!("No providers available for balanced selection");
            LLMError::Internal("No providers available".to_string())
        })
    }
    
    /// Select most reliable model (lowest error rate)
    async fn select_most_reliable(&self) -> LLMResult<String> {
        let health_status = self.health_status.read().await;
        
        let mut most_reliable_model = None;
        let mut lowest_error_rate = f64::MAX;
        
        for (provider_type, config) in &self.provider_configs {
            if let Some(status) = health_status.get(provider_type) {
                if status.is_healthy && status.error_rate < lowest_error_rate {
                    lowest_error_rate = status.error_rate;
                    if let Some(model) = config.models.first() {
                        most_reliable_model = Some(model.id.clone());
                    }
                }
            }
        }
        
        most_reliable_model.ok_or_else(|| {
            error!("No healthy providers available for reliability optimization");
            LLMError::Internal("No healthy providers available".to_string())
        })
    }
    
    /// Select best model for specific task
    async fn select_best_for_task(&self, task: &str) -> LLMResult<String> {
        // For now, use simple task-based selection
        // In the future, this could use model capability matching
        match task {
            "coding" => {
                // Prefer models with code generation capabilities
                for (provider_type, config) in &self.provider_configs {
                    if self.is_provider_healthy(provider_type).await {
                        for model in &config.models {
                            if model.capabilities.contains(&ModelCapability::CodeGeneration) {
                                return Ok(model.id.clone());
                            }
                        }
                    }
                }
                // Fallback to balanced selection
                self.select_balanced().await
            },
            "analysis" => {
                // Prefer models with reasoning capabilities
                for (provider_type, config) in &self.provider_configs {
                    if self.is_provider_healthy(provider_type).await {
                        for model in &config.models {
                            if model.capabilities.contains(&ModelCapability::Reasoning) {
                                return Ok(model.id.clone());
                            }
                        }
                    }
                }
                self.select_balanced().await
            },
            _ => self.select_balanced().await
        }
    }
    
    /// Select default model (fallback)
    async fn select_default(&self) -> LLMResult<String> {
        // Return the first available healthy model
        for (provider_type, config) in &self.provider_configs {
            if self.is_provider_healthy(provider_type).await {
                if let Some(model) = config.models.first() {
                    return Ok(model.id.clone());
                }
            }
        }
        
        // If no healthy providers, try Anthropic as fallback
        warn!("No healthy providers found, falling back to Anthropic");
        if let Some(anthropic_config) = self.provider_configs.get(&LLMProviderType::Anthropic) {
            if let Some(model) = anthropic_config.models.first() {
                return Ok(model.id.clone());
            }
        }
        
        Err(LLMError::Internal("No providers available".to_string()))
    }
    
    /// Check if provider is healthy (uses existing health status)
    async fn is_provider_healthy(&self, provider_type: &LLMProviderType) -> bool {
        let health_status = self.health_status.read().await;
        health_status
            .get(provider_type)
            .map(|status| status.is_healthy)
            .unwrap_or(true) // Default to healthy for demo purposes when no status available
    }
    
    /// Check if model exists in any provider
    async fn has_model(&self, model_name: &str) -> bool {
        for config in self.provider_configs.values() {
            if config.models.iter().any(|m| m.id == model_name) {
                return true;
            }
        }
        false
    }
    
    /// Check if model meets user constraints
    async fn model_meets_constraints(&self, model_name: &str, config: &CircuitBreakerConfig) -> bool {
        // Find the model and check constraints
        for provider_config in self.provider_configs.values() {
            if let Some(model) = provider_config.models.iter().find(|m| m.id == model_name) {
                // Check cost constraint
                if let Some(max_cost) = config.max_cost_per_1k_tokens {
                    let total_cost = (model.cost_per_input_token + model.cost_per_output_token) * 1000.0;
                    if total_cost > max_cost {
                        return false;
                    }
                }
                
                // Check latency constraint
                if let Some(max_latency) = config.max_latency_ms {
                    let health_status = futures::executor::block_on(self.health_status.read());
                    for (provider_type, provider_config) in &self.provider_configs {
                        if provider_config.models.iter().any(|m| m.id == model_name) {
                            if let Some(status) = health_status.get(provider_type) {
                                if status.average_latency_ms > max_latency {
                                    return false;
                                }
                            }
                        }
                    }
                }
                
                return true;
            }
        }
        false
    }
    
    /// Find model that meets constraints
    async fn find_model_with_constraints(&self, config: &CircuitBreakerConfig) -> LLMResult<String> {
        let health_status = self.health_status.read().await;
        
        for (provider_type, provider_config) in &self.provider_configs {
            if let Some(status) = health_status.get(provider_type) {
                if !status.is_healthy {
                    continue;
                }
                
                for model in &provider_config.models {
                    let mut meets_constraints = true;
                    
                    // Check cost constraint
                    if let Some(max_cost) = config.max_cost_per_1k_tokens {
                        let total_cost = (model.cost_per_input_token + model.cost_per_output_token) * 1000.0;
                        if total_cost > max_cost {
                            meets_constraints = false;
                        }
                    }
                    
                    // Check latency constraint
                    if let Some(max_latency) = config.max_latency_ms {
                        if status.average_latency_ms > max_latency {
                            meets_constraints = false;
                        }
                    }
                    
                    if meets_constraints {
                        return Ok(model.id.clone());
                    }
                }
            }
        }
        
        // If no model meets constraints, use balanced selection
        warn!("No models meet specified constraints, using balanced selection");
        self.select_balanced().await
    }
}

/// Simple API key storage
pub struct SimpleApiKeyStorage {
    keys: Arc<RwLock<HashMap<String, String>>>,
}

impl SimpleApiKeyStorage {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store_key(&self, key_id: &str, key: &str) {
        let mut keys = self.keys.write().await;
        keys.insert(key_id.to_string(), key.to_string());
    }

    pub async fn get_key(&self, key_id: &str) -> Option<String> {
        let keys = self.keys.read().await;
        keys.get(key_id).cloned()
    }
}
