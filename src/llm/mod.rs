//! LLM Provider Routing and Management
//! 
//! This module provides the core infrastructure for LLM provider routing,
//! following the OpenRouter Alternative architecture with BYOK (Bring Your Own Key) model.

pub mod providers;
pub mod router;
pub mod streaming;
pub mod security;
pub mod cost;
pub mod traits;
pub mod sse;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// LLM Provider types supported by Circuit Breaker
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LLMProviderType {
    OpenAI,
    Anthropic,
    Google,
    Cohere,
    Mistral,
    Perplexity,
    Groq,
    Together,
    Replicate,
    Ollama,
    VLLM,
    Custom(String),
}

impl std::fmt::Display for LLMProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProviderType::OpenAI => write!(f, "openai"),
            LLMProviderType::Anthropic => write!(f, "anthropic"),
            LLMProviderType::Google => write!(f, "google"),
            LLMProviderType::Cohere => write!(f, "cohere"),
            LLMProviderType::Mistral => write!(f, "mistral"),
            LLMProviderType::Perplexity => write!(f, "perplexity"),
            LLMProviderType::Groq => write!(f, "groq"),
            LLMProviderType::Together => write!(f, "together"),
            LLMProviderType::Replicate => write!(f, "replicate"),
            LLMProviderType::Ollama => write!(f, "ollama"),
            LLMProviderType::VLLM => write!(f, "vllm"),
            LLMProviderType::Custom(name) => write!(f, "custom-{}", name),
        }
    }
}

// Re-export main types for easier access
pub use router::LLMRouter;
pub use traits::{
    LLMProviderClient, ModelInfo, ProviderConfig, ProviderConfigRequirements,
    AuthMethod, RateLimitInfo, ParameterRestriction, ModelCapability,
    ProviderFactory, CostCalculator, CostBreakdown, ProviderHealth,
    ProviderRegistry
};
pub use cost::{CostOptimizer, BudgetManager, CostAnalyzer, InMemoryUsageTracker};

// Re-export streaming types
pub use streaming::{StreamEvent, StreamingSession, StreamingProtocol};

/// LLM Provider configuration with secure key management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProvider {
    pub id: Uuid,
    pub provider_type: LLMProviderType,
    pub name: String,
    pub base_url: String,
    pub api_key_id: Option<String>, // Reference to secure key storage
    pub models: Vec<LLMModel>,
    pub rate_limits: RateLimits,
    pub health_status: ProviderHealthStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// LLM Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMModel {
    pub id: String,
    pub name: String,
    pub provider_id: Uuid,
    pub max_tokens: u32,
    pub context_window: u32,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub capabilities: Vec<ModelCapability>,
}

// ModelCapability is now defined in traits.rs

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub concurrent_requests: u32,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthStatus {
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub error_rate: f64,
    pub average_latency_ms: u64,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

/// Routing strategy for LLM requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    CostOptimized,
    PerformanceFirst,
    LoadBalanced,
    FailoverChain,
    ModelSpecific(String),
    Custom(String),
}

/// LLM Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub id: Uuid,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub functions: Option<Vec<FunctionDefinition>>,
    pub function_call: Option<String>,
    pub user: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Embeddings request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsRequest {
    pub id: Uuid,
    pub model: String,
    pub input: EmbeddingsInput,
    pub user: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Input for embeddings generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingsInput {
    /// Single text string
    Text(String),
    /// Array of text strings
    TextArray(Vec<String>),
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub function_call: Option<FunctionCall>,
}

/// Message roles
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Function,
}

impl<'de> Deserialize<'de> for MessageRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "system" => Ok(MessageRole::System),
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "function" => Ok(MessageRole::Function),
            _ => Err(serde::de::Error::unknown_variant(&s, &["system", "user", "assistant", "function"])),
        }
    }
}

/// Function definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Function call structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// LLM Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: TokenUsage,
    pub provider: LLMProviderType,
    pub routing_info: RoutingInfo,
}

/// Embeddings response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub data: Vec<EmbeddingData>,
    pub usage: EmbeddingsUsage,
    pub provider: LLMProviderType,
    pub routing_info: RoutingInfo,
}

/// Single embedding data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub index: u32,
    pub embedding: Vec<f64>,
    pub object: String,
}

/// Token usage for embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: f64,
}

/// Response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: f64,
}

/// Routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInfo {
    pub selected_provider: LLMProviderType,
    pub routing_strategy: RoutingStrategy,
    pub latency_ms: u64,
    pub retry_count: u32,
    pub fallback_used: bool,
    pub provider_used: LLMProviderType,
    pub total_latency_ms: u64,
    pub provider_latency_ms: u64,
}

/// Streaming chunk for real-time responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub id: String,
    pub object: String,
    pub choices: Vec<StreamingChoice>,
    pub created: u64,
    pub model: String,
    pub provider: LLMProviderType,
}

/// Streaming choice for chunked responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChoice {
    pub index: u32,
    pub delta: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Delta content for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDelta {
    pub role: MessageRole,
    pub content: String,
}



/// Legacy streaming chunk (keeping for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyStreamingChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LegacyStreamingChoice>,
    pub provider: LLMProviderType,
}

/// Legacy streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyStreamingChoice {
    pub index: u32,
    pub delta: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Cost tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    pub request_id: Uuid,
    pub provider: LLMProviderType,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub project_id: Option<String>,
}

/// Error types for LLM operations  
#[derive(Debug, Clone, thiserror::Error)]
pub enum LLMError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("Model not supported: {0}")]
    ModelNotSupported(String),
    
    #[error("Rate limit exceeded for provider: {0}")]
    RateLimitExceeded(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Provider health check failed: {0}")]
    ProviderUnhealthy(String),
    
    #[error("Request timeout: {0}")]
    Timeout(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
}

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// LLM Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRouterConfig {
    pub default_strategy: RoutingStrategy,
    pub fallback_enabled: bool,
    pub health_check_interval_seconds: u64,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub cost_optimization_enabled: bool,
    pub performance_targets: PerformanceTargets,
}

/// Performance targets for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTargets {
    pub max_latency_ms: u64,
    pub min_success_rate: f64,
    pub max_error_rate: f64,
}

impl Default for LLMRouterConfig {
    fn default() -> Self {
        Self {
            default_strategy: RoutingStrategy::CostOptimized,
            fallback_enabled: true,
            health_check_interval_seconds: 60,
            max_retries: 3,
            timeout_seconds: 30,
            cost_optimization_enabled: true,
            performance_targets: PerformanceTargets {
                max_latency_ms: 5000,
                min_success_rate: 0.95,
                max_error_rate: 0.05,
            },
        }
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 1000,
            tokens_per_minute: 100000,
            concurrent_requests: 10,
        }
    }
}

impl Default for ProviderHealthStatus {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_check: Utc::now(),
            error_rate: 0.0,
            average_latency_ms: 0,
            consecutive_failures: 0,
            last_error: None,
        }
    }
}