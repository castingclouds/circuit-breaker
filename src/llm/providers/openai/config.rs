//! OpenAI provider configuration
//! This module contains configuration structures and defaults specific to OpenAI

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm::{
    traits::{
        AuthMethod, ModelCapability, ModelInfo, ParameterRestriction, ProviderConfig,
        ProviderConfigRequirements, RateLimitInfo,
    },
    LLMProviderType,
};

/// OpenAI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// API key for authentication
    pub api_key: String,
    /// Base URL for API requests
    pub base_url: String,
    /// Organization ID (optional)
    pub organization: Option<String>,
    /// Project ID (optional)
    pub project: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        let default_model =
            std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string());
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            project: None,
            default_model,
            timeout_seconds: 30,
            max_retries: 3,
            custom_headers: HashMap::new(),
        }
    }
}

/// Get OpenAI provider configuration requirements
pub fn get_config_requirements() -> ProviderConfigRequirements {
    let mut parameter_restrictions = HashMap::new();

    // o4 models have specific parameter restrictions
    parameter_restrictions.insert(
        "temperature".to_string(),
        ParameterRestriction::Custom("o4 models only support temperature=1.0".to_string()),
    );

    parameter_restrictions.insert(
        "max_tokens".to_string(),
        ParameterRestriction::Custom("o4 models use max_completion_tokens instead".to_string()),
    );

    ProviderConfigRequirements {
        api_key_env_var: "OPENAI_API_KEY".to_string(),
        base_url_env_var: Some("OPENAI_BASE_URL".to_string()),
        auth_methods: vec![AuthMethod::BearerToken],
        rate_limits: Some(RateLimitInfo {
            requests_per_minute: Some(3500), // Tier 4 default
            tokens_per_minute: Some(200_000),
            requests_per_day: None,
            concurrent_requests: Some(500),
        }),
        parameter_restrictions,
    }
}

/// Get default OpenAI provider configuration
pub fn get_default_config() -> ProviderConfig {
    let default_model =
        std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string());
    ProviderConfig {
        provider_type: LLMProviderType::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        default_model,
        models: get_available_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 1, // High priority
    }
}

/// Get available OpenAI models with their configurations
/// Only loads the default model from environment to avoid hardcoded non-existent models
pub fn get_available_models() -> Vec<ModelInfo> {
    let default_model =
        std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string());

    // Create a single model info for the default model from environment
    let model_info = ModelInfo {
        id: default_model.clone(),
        name: format!("OpenAI {}", default_model),
        provider: LLMProviderType::OpenAI,
        context_window: 8192,
        max_output_tokens: 4096,
        supports_streaming: true,
        supports_function_calling: true,
        cost_per_input_token: 0.00003, // Default to GPT-4 pricing
        cost_per_output_token: 0.00006,
        capabilities: vec![
            ModelCapability::TextGeneration,
            ModelCapability::CodeGeneration,
            ModelCapability::ConversationalAI,
            ModelCapability::FunctionCalling,
            ModelCapability::ReasoningChain,
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

/// Check if a model is an o4 series model
pub fn is_o4_model(model: &str) -> bool {
    model.starts_with("o4-")
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
