//! OpenAI provider configuration
//! This module contains configuration structures and defaults specific to OpenAI

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::llm::{LLMProviderType, traits::{
    ModelInfo, ProviderConfig, ProviderConfigRequirements, AuthMethod, 
    RateLimitInfo, ParameterRestriction, ModelCapability
}};

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
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            project: None,
            default_model: "gpt-4".to_string(),
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
        ParameterRestriction::Custom("o4 models only support temperature=1.0".to_string())
    );
    
    parameter_restrictions.insert(
        "max_tokens".to_string(),
        ParameterRestriction::Custom("o4 models use max_completion_tokens instead".to_string())
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
    ProviderConfig {
        provider_type: LLMProviderType::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        default_model: "gpt-4".to_string(),
        models: get_available_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 1, // High priority
    }
}

/// Get available OpenAI models with their configurations
pub fn get_available_models() -> Vec<ModelInfo> {
    vec![
        // GPT-4 models
        ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider: LLMProviderType::OpenAI,
            context_window: 8192,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.00003,  // $0.03 per 1K tokens
            cost_per_output_token: 0.00006, // $0.06 per 1K tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::CodeGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::FunctionCalling,
                ModelCapability::ReasoningChain,
            ],
            parameter_restrictions: HashMap::new(),
        },
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            name: "GPT-4 Turbo".to_string(),
            provider: LLMProviderType::OpenAI,
            context_window: 128000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.00001,  // $0.01 per 1K tokens
            cost_per_output_token: 0.00003, // $0.03 per 1K tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::CodeGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::FunctionCalling,
                ModelCapability::VisionInput,
                ModelCapability::JsonMode,
                ModelCapability::ReasoningChain,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // GPT-3.5 models
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            name: "GPT-3.5 Turbo".to_string(),
            provider: LLMProviderType::OpenAI,
            context_window: 16384,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.000001,  // $0.001 per 1K tokens
            cost_per_output_token: 0.000002, // $0.002 per 1K tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::CodeGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::FunctionCalling,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // o4 models (latest reasoning models)
        ModelInfo {
            id: "o4-mini-2025-04-16".to_string(),
            name: "o4-mini (April 2025)".to_string(),
            provider: LLMProviderType::OpenAI,
            context_window: 128000,
            max_output_tokens: 65536,
            supports_streaming: true,
            supports_function_calling: false, // o4 models don't support function calling yet
            cost_per_input_token: 0.000003,  // $0.003 per 1K tokens
            cost_per_output_token: 0.000012, // $0.012 per 1K tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::CodeGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::ReasoningChain,
            ],
            parameter_restrictions: {
                let mut restrictions = HashMap::new();
                restrictions.insert(
                    "temperature".to_string(),
                    ParameterRestriction::Fixed(serde_json::Value::Number(serde_json::Number::from(1)))
                );
                restrictions.insert(
                    "max_tokens".to_string(),
                    ParameterRestriction::Custom("Use max_completion_tokens instead".to_string())
                );
                restrictions
            },
        },
        ModelInfo {
            id: "o4-2025-04-16".to_string(),
            name: "o4 (April 2025)".to_string(),
            provider: LLMProviderType::OpenAI,
            context_window: 128000,
            max_output_tokens: 65536,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.000015,  // $0.015 per 1K tokens
            cost_per_output_token: 0.00006,  // $0.06 per 1K tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::CodeGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::ReasoningChain,
            ],
            parameter_restrictions: {
                let mut restrictions = HashMap::new();
                restrictions.insert(
                    "temperature".to_string(),
                    ParameterRestriction::Fixed(serde_json::Value::Number(serde_json::Number::from(1)))
                );
                restrictions.insert(
                    "max_tokens".to_string(),
                    ParameterRestriction::Custom("Use max_completion_tokens instead".to_string())
                );
                restrictions
            },
        },
    ]
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