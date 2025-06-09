//! Google provider configuration
//! This module contains configuration structures and defaults specific to Google Gemini

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::llm::{LLMProviderType, traits::{
    ModelInfo, ProviderConfig, ProviderConfigRequirements, AuthMethod, 
    RateLimitInfo, ParameterRestriction, ModelCapability
}};

/// Google-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
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
    /// Project ID (optional)
    pub project_id: Option<String>,
    /// API version to use
    pub api_version: String,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            default_model: "gemini-pro".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            custom_headers: HashMap::new(),
            project_id: None,
            api_version: "v1beta".to_string(),
        }
    }
}

/// Get Google provider configuration requirements
pub fn get_config_requirements() -> ProviderConfigRequirements {
    let parameter_restrictions = HashMap::new();
    
    ProviderConfigRequirements {
        api_key_env_var: "GOOGLE_API_KEY".to_string(),
        base_url_env_var: Some("GOOGLE_BASE_URL".to_string()),
        auth_methods: vec![AuthMethod::ApiKey],
        rate_limits: Some(RateLimitInfo {
            requests_per_minute: Some(60), // Free tier default
            tokens_per_minute: Some(32000),
            requests_per_day: None,
            concurrent_requests: Some(10),
        }),
        parameter_restrictions,
    }
}

/// Get default Google provider configuration
pub fn get_default_config() -> ProviderConfig {
    ProviderConfig {
        provider_type: LLMProviderType::Google,
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        default_model: "gemini-pro".to_string(),
        models: get_available_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 3, // Medium priority
    }
}

/// Get available Google models with their configurations
pub fn get_available_models() -> Vec<ModelInfo> {
    vec![
        // Gemini Pro models
        ModelInfo {
            id: "gemini-pro".to_string(),
            name: "Gemini Pro".to_string(),
            provider: LLMProviderType::Google,
            context_window: 32768,
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.0000005,  // $0.50 per 1M tokens
            cost_per_output_token: 0.0000015, // $1.50 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::CodeGeneration,
                ModelCapability::ReasoningChain,
                ModelCapability::Translation,
                ModelCapability::Summarization,
                ModelCapability::FunctionCalling,
            ],
            parameter_restrictions: HashMap::new(),
        },
        ModelInfo {
            id: "gemini-pro-vision".to_string(),
            name: "Gemini Pro Vision".to_string(),
            provider: LLMProviderType::Google,
            context_window: 16384,
            max_output_tokens: 2048,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.00000025, // $0.25 per 1M tokens
            cost_per_output_token: 0.0000005,  // $0.50 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::Vision,
                ModelCapability::Multimodal,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // Gemini 1.5 models
        ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            provider: LLMProviderType::Google,
            context_window: 2097152, // 2M tokens
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.0000035,  // $3.50 per 1M tokens
            cost_per_output_token: 0.0000105, // $10.50 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::CodeGeneration,
                ModelCapability::ReasoningChain,
                ModelCapability::Translation,
                ModelCapability::Summarization,
                ModelCapability::FunctionCalling,
                ModelCapability::Vision,
                ModelCapability::Audio,
                ModelCapability::Multimodal,
            ],
            parameter_restrictions: HashMap::new(),
        },
        ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            name: "Gemini 1.5 Flash".to_string(),
            provider: LLMProviderType::Google,
            context_window: 1048576, // 1M tokens
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.000000075, // $0.075 per 1M tokens
            cost_per_output_token: 0.0000003,  // $0.30 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::CodeGeneration,
                ModelCapability::ReasoningChain,
                ModelCapability::Translation,
                ModelCapability::Summarization,
                ModelCapability::FunctionCalling,
                ModelCapability::Vision,
                ModelCapability::Audio,
                ModelCapability::Multimodal,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // Gemini 2.0 models (experimental/preview)
        ModelInfo {
            id: "gemini-2.0-flash-exp".to_string(),
            name: "Gemini 2.0 Flash (Experimental)".to_string(),
            provider: LLMProviderType::Google,
            context_window: 1048576, // 1M tokens
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.000000075, // $0.075 per 1M tokens
            cost_per_output_token: 0.0000003,  // $0.30 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::CodeGeneration,
                ModelCapability::ReasoningChain,
                ModelCapability::Translation,
                ModelCapability::Summarization,
                ModelCapability::FunctionCalling,
                ModelCapability::Vision,
                ModelCapability::Audio,
                ModelCapability::Multimodal,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // Custom test models (for testing)
        ModelInfo {
            id: "gemini-2.5-flash-preview-05-20".to_string(),
            name: "Gemini 2.5 Flash Preview (Custom)".to_string(),
            provider: LLMProviderType::Google,
            context_window: 1048576, // 1M tokens
            max_output_tokens: 8192,
            supports_streaming: true,
            supports_function_calling: true,
            cost_per_input_token: 0.000000075, // $0.075 per 1M tokens
            cost_per_output_token: 0.0000003,  // $0.30 per 1M tokens
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::CodeGeneration,
                ModelCapability::ReasoningChain,
                ModelCapability::Translation,
                ModelCapability::Summarization,
                ModelCapability::FunctionCalling,
                ModelCapability::Vision,
                ModelCapability::Audio,
                ModelCapability::Multimodal,
            ],
            parameter_restrictions: HashMap::new(),
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

/// Check if a model is a Gemini model
pub fn is_gemini_model(model: &str) -> bool {
    model.starts_with("gemini-") || model == "gemini-pro"
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

/// Check if model supports vision capabilities
pub fn supports_vision(model: &str) -> bool {
    model_supports_capability(model, &ModelCapability::Vision)
}

/// Check if model supports multimodal capabilities
pub fn supports_multimodal(model: &str) -> bool {
    model_supports_capability(model, &ModelCapability::Multimodal)
}

/// Check if model supports function calling
pub fn supports_function_calling(model: &str) -> bool {
    model_supports_capability(model, &ModelCapability::FunctionCalling)
}

/// Get the maximum context window for a model
pub fn get_context_window(model: &str) -> u32 {
    let models = get_available_models();
    models
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.context_window)
        .unwrap_or(32768) // Default to Gemini Pro context window
}

/// Get the maximum output tokens for a model
pub fn get_max_output_tokens(model: &str) -> u32 {
    let models = get_available_models();
    models
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.max_output_tokens)
        .unwrap_or(8192) // Default to common max output
}

/// Validate Google API key format
pub fn validate_api_key(api_key: &str) -> bool {
    // Google API keys typically start with "AIza" and are 39 characters long
    api_key.starts_with("AIza") && api_key.len() == 39
}

/// Get safety settings for Google API
pub fn get_default_safety_settings() -> Vec<super::types::GoogleSafetySetting> {
    vec![
        super::types::GoogleSafetySetting {
            category: "HARM_CATEGORY_HARASSMENT".to_string(),
            threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
        },
        super::types::GoogleSafetySetting {
            category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
            threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
        },
        super::types::GoogleSafetySetting {
            category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
            threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
        },
        super::types::GoogleSafetySetting {
            category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
            threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
        },
    ]
}