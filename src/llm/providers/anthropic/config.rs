//! Anthropic provider configuration
//! This module contains configuration structures and defaults specific to Anthropic

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm::{
    traits::{
        AuthMethod, ModelCapability, ModelInfo, ParameterRestriction, ProviderConfig,
        ProviderConfigRequirements, RateLimitInfo,
    },
    LLMProviderType,
};

/// Anthropic-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for API requests
    pub base_url: String,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,
    /// API version to use
    pub api_version: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        let default_model = std::env::var("ANTHROPIC_DEFAULT_MODEL")
            .unwrap_or_else(|_| "claude-3-sonnet-20240229".to_string());
        Self {
            api_key: String::new(),
            base_url: "https://api.anthropic.com".to_string(),
            default_model,
            timeout_seconds: 30,
            max_retries: 3,
            custom_headers: HashMap::new(),
            api_version: "2023-06-01".to_string(),
        }
    }
}

/// Get Anthropic provider configuration requirements
pub fn get_config_requirements() -> ProviderConfigRequirements {
    let parameter_restrictions = HashMap::new();

    ProviderConfigRequirements {
        api_key_env_var: "ANTHROPIC_API_KEY".to_string(),
        base_url_env_var: Some("ANTHROPIC_BASE_URL".to_string()),
        auth_methods: vec![AuthMethod::ApiKey],
        rate_limits: Some(RateLimitInfo {
            requests_per_minute: Some(1000), // Tier 4 default
            tokens_per_minute: Some(400_000),
            requests_per_day: None,
            concurrent_requests: Some(100),
        }),
        parameter_restrictions,
    }
}

/// Get default Anthropic provider configuration
pub fn get_default_config() -> ProviderConfig {
    let default_model = std::env::var("ANTHROPIC_DEFAULT_MODEL")
        .unwrap_or_else(|_| "claude-3-sonnet-20240229".to_string());
    ProviderConfig {
        provider_type: LLMProviderType::Anthropic,
        base_url: "https://api.anthropic.com".to_string(),
        default_model,
        models: get_available_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 2, // High priority
    }
}

/// Get available Anthropic models with their configurations
/// Only loads the default model from environment to avoid hardcoded non-existent models
pub fn get_available_models() -> Vec<ModelInfo> {
    let default_model = std::env::var("ANTHROPIC_DEFAULT_MODEL")
        .unwrap_or_else(|_| "claude-3-sonnet-20240229".to_string());

    // Create a single model info for the default model from environment
    let model_info = ModelInfo {
        id: default_model.clone(),
        name: format!("Anthropic {}", default_model),
        provider: LLMProviderType::Anthropic,
        context_window: 200000,
        max_output_tokens: 4096,
        supports_streaming: true,
        supports_function_calling: true,
        cost_per_input_token: 0.000003, // Default to Claude 3 Sonnet pricing
        cost_per_output_token: 0.000015,
        capabilities: vec![
            ModelCapability::TextGeneration,
            ModelCapability::ConversationalAI,
            ModelCapability::CodeGeneration,
            ModelCapability::ReasoningChain,
            ModelCapability::Translation,
            ModelCapability::Summarization,
        ],
        parameter_restrictions: HashMap::new(),
    };

    vec![model_info]
}

/// Check if a model has specific parameter restrictions
pub fn has_parameter_restriction(model: &str, parameter: &str) -> Option<ParameterRestriction> {
    let models = get_available_models();
    models
        .iter()
        .find(|m| m.id == model)
        .and_then(|m| m.parameter_restrictions.get(parameter))
        .cloned()
}

/// Check if a model is a Claude model
pub fn is_claude_model(model: &str) -> bool {
    model.starts_with("claude-")
}

/// Check if a model supports a specific capability
pub fn model_supports_capability(model: &str, capability: &ModelCapability) -> bool {
    let models = get_available_models();
    models
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.capabilities.contains(capability))
        .unwrap_or(false)
}

/// Get cost information for a model
pub fn get_model_cost_info(model: &str) -> Option<(f64, f64)> {
    let models = get_available_models();
    models
        .iter()
        .find(|m| m.id == model)
        .map(|m| (m.cost_per_input_token, m.cost_per_output_token))
}

/// Get the system prompt format for Anthropic models
pub fn format_system_prompt(system_content: &str) -> String {
    // Anthropic expects system prompts in a specific format
    system_content.to_string()
}

/// Check if model supports function calling
pub fn supports_function_calling(model: &str) -> bool {
    model_supports_capability(model, &ModelCapability::FunctionCalling)
}
