//! Common traits and types for LLM providers
//! This module defines the core interfaces that all LLM providers must implement

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    LLMError, LLMRequest, LLMResponse, LLMResult, StreamingChunk,
    LLMProviderType, TokenUsage, ChatMessage
};

/// Core trait that all LLM provider clients must implement
#[async_trait]
pub trait LLMProviderClient: Send + Sync {
    /// Send a chat completion request
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse>;

    /// Send a streaming chat completion request
    async fn chat_completion_stream(
        &self,
        request: LLMRequest,
        api_key: String,
    ) -> LLMResult<Box<dyn futures::Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin>>;

    /// Get the provider type
    fn provider_type(&self) -> LLMProviderType;

    /// Health check for the provider
    async fn health_check(&self, api_key: &str) -> LLMResult<bool>;

    /// Get available models for this provider
    fn get_available_models(&self) -> Vec<ModelInfo>;

    /// Validate if a model is supported by this provider
    fn supports_model(&self, model: &str) -> bool;

    /// Get provider-specific configuration requirements
    fn get_config_requirements(&self) -> ProviderConfigRequirements;
}

/// Configuration requirements for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigRequirements {
    /// Required API key environment variable name
    pub api_key_env_var: String,
    /// Optional base URL override
    pub base_url_env_var: Option<String>,
    /// Supported authentication methods
    pub auth_methods: Vec<AuthMethod>,
    /// Rate limiting information
    pub rate_limits: Option<RateLimitInfo>,
    /// Special parameter requirements
    pub parameter_restrictions: HashMap<String, ParameterRestriction>,
}

/// Authentication methods supported by a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    BearerToken,
    ApiKey,
    OAuth,
    Custom(String),
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub requests_per_minute: Option<u32>,
    pub tokens_per_minute: Option<u32>,
    pub requests_per_day: Option<u32>,
    pub concurrent_requests: Option<u32>,
}

/// Parameter restrictions for specific models or providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterRestriction {
    /// Fixed value - parameter must be this exact value
    Fixed(serde_json::Value),
    /// Range - parameter must be within this range
    Range { min: f64, max: f64 },
    /// Enum - parameter must be one of these values
    Enum(Vec<serde_json::Value>),
    /// Not supported - parameter is not supported
    NotSupported,
    /// Custom validation rule
    Custom(String),
}

/// Model information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier
    pub id: String,
    /// Human-readable model name
    pub name: String,
    /// Provider that hosts this model
    pub provider: LLMProviderType,
    /// Maximum context window size
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output_tokens: u32,
    /// Whether the model supports streaming
    pub supports_streaming: bool,
    /// Whether the model supports function calling
    pub supports_function_calling: bool,
    /// Cost per input token (in USD)
    pub cost_per_input_token: f64,
    /// Cost per output token (in USD)
    pub cost_per_output_token: f64,
    /// Model capabilities
    pub capabilities: Vec<ModelCapability>,
    /// Model-specific parameter restrictions
    pub parameter_restrictions: HashMap<String, ParameterRestriction>,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCapability {
    TextGeneration,
    CodeGeneration,
    ConversationalAI,
    FunctionCalling,
    JsonMode,
    VisionInput,
    AudioInput,
    ImageGeneration,
    Embedding,
    Classification,
    Summarization,
    Translation,
    ReasoningChain,
    TextAnalysis,
    Reasoning,
    QuestionAnswering,
    Vision,
    Audio,
    Multimodal,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type
    pub provider_type: LLMProviderType,
    /// Base URL for API requests
    pub base_url: String,
    /// Default model for this provider
    pub default_model: String,
    /// Available models
    pub models: Vec<ModelInfo>,
    /// Provider-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Priority for provider selection (lower = higher priority)
    pub priority: u32,
}

/// Factory trait for creating provider clients
pub trait ProviderFactory: Send + Sync {
    /// Create a new provider client instance
    fn create_client(&self, config: &ProviderConfig) -> Box<dyn LLMProviderClient>;
    
    /// Get the provider type this factory creates
    fn provider_type(&self) -> LLMProviderType;
    
    /// Get default configuration for this provider
    fn default_config(&self) -> ProviderConfig;
}

/// Trait for cost calculation
pub trait CostCalculator: Send + Sync {
    /// Calculate cost for a completed request
    fn calculate_cost(&self, usage: &TokenUsage, model: &str) -> f64;
    
    /// Estimate cost for a request before sending
    fn estimate_cost(&self, input_tokens: u32, estimated_output_tokens: u32, model: &str) -> f64;
    
    /// Get cost breakdown by token type
    fn get_cost_breakdown(&self, usage: &TokenUsage, model: &str) -> CostBreakdown;
}

/// Cost breakdown structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider: LLMProviderType,
    pub is_healthy: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<u64>,
    pub error_rate: f64,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

/// Provider registry for managing multiple providers
pub trait ProviderRegistry: Send + Sync {
    /// Register a new provider
    fn register_provider(&mut self, factory: Box<dyn ProviderFactory>);
    
    /// Get a provider client by type
    fn get_provider(&self, provider_type: &LLMProviderType) -> Option<&dyn LLMProviderClient>;
    
    /// Get all available providers
    fn get_all_providers(&self) -> Vec<&dyn LLMProviderClient>;
    
    /// Check if a provider is available
    fn is_provider_available(&self, provider_type: &LLMProviderType) -> bool;
    
    /// Get provider health status
    fn get_provider_health(&self, provider_type: &LLMProviderType) -> Option<ProviderHealth>;
}