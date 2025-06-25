//! Ollama provider configuration
//! This module contains configuration structures and defaults specific to Ollama

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm::{
    traits::{
        ModelCapability, ModelInfo, ParameterRestriction, ProviderConfig,
        ProviderConfigRequirements, RateLimitInfo,
    },
    LLMProviderType,
};

/// Ollama-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Base URL for Ollama API (typically http://localhost:11434)
    pub base_url: String,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Whether to verify SSL certificates (useful for self-signed certs)
    pub verify_ssl: bool,
    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,
    /// Keep alive setting for models (e.g., "5m", "1h", or "-1" for indefinite)
    pub keep_alive: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            default_model: "qwen2.5-coder:3b".to_string(),
            timeout_seconds: 60, // Longer timeout for local inference
            max_retries: 2,
            verify_ssl: true,
            custom_headers: HashMap::new(),
            keep_alive: "5m".to_string(),
        }
    }
}

/// Get Ollama provider configuration requirements
pub fn get_config_requirements() -> ProviderConfigRequirements {
    let mut parameter_restrictions = HashMap::new();

    // Ollama uses different parameter names than OpenAI
    parameter_restrictions.insert(
        "max_tokens".to_string(),
        ParameterRestriction::Custom("Use num_predict instead".to_string()),
    );

    parameter_restrictions.insert(
        "frequency_penalty".to_string(),
        ParameterRestriction::NotSupported,
    );

    parameter_restrictions.insert(
        "presence_penalty".to_string(),
        ParameterRestriction::NotSupported,
    );

    ProviderConfigRequirements {
        api_key_env_var: "OLLAMA_API_KEY".to_string(), // Optional, often not needed
        base_url_env_var: Some("OLLAMA_BASE_URL".to_string()),
        auth_methods: vec![], // Ollama typically doesn't require authentication
        rate_limits: Some(RateLimitInfo {
            requests_per_minute: None, // No hard limits for local instance
            tokens_per_minute: None,
            requests_per_day: None,
            concurrent_requests: Some(1), // Usually limited by hardware
        }),
        parameter_restrictions,
    }
}

/// Get default Ollama provider configuration
pub fn get_default_config() -> ProviderConfig {
    ProviderConfig {
        provider_type: LLMProviderType::Ollama,
        base_url: "http://localhost:11434".to_string(),
        default_model: "qwen2.5-coder:3b".to_string(),
        models: get_default_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 3, // Lower priority than cloud providers by default
    }
}

/// Get default Ollama models (based on commonly available models)
/// Note: In a real implementation, these would be fetched dynamically from /api/tags
pub fn get_default_models() -> Vec<ModelInfo> {
    vec![
        // User's specific models
        ModelInfo {
            id: "qwen2.5-coder:3b".to_string(),
            name: "Qwen2.5 Coder 3B".to_string(),
            provider: LLMProviderType::Ollama,
            context_window: 32768,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::CodeGeneration,
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::Reasoning,
            ],
            parameter_restrictions: HashMap::new(),
        },
        ModelInfo {
            id: "gemma3:4b".to_string(),
            name: "Gemma 3 4B".to_string(),
            provider: LLMProviderType::Ollama,
            context_window: 8192,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::Reasoning,
            ],
            parameter_restrictions: HashMap::new(),
        },
        ModelInfo {
            id: "nomic-embed-text:latest".to_string(),
            name: "Nomic Embed Text".to_string(),
            provider: LLMProviderType::Ollama,
            context_window: 8192,
            max_output_tokens: 0,      // Embedding models don't generate text
            supports_streaming: false, // Embeddings are not streamed
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![ModelCapability::Embedding],
            parameter_restrictions: HashMap::new(),
        },
    ]
}

/// Check if a model supports a specific capability
pub fn model_supports_capability(model: &str, capability: &ModelCapability) -> bool {
    let models = get_default_models();
    models
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.capabilities.contains(capability))
        .unwrap_or(false)
}

/// Get model information by ID
pub fn get_model_info(model: &str) -> Option<ModelInfo> {
    let models = get_default_models();
    models.into_iter().find(|m| m.id == model)
}

/// Check if a model is a code-focused model
pub fn is_code_model(model: &str) -> bool {
    model.starts_with("codellama") || model == "phi" || model.starts_with("qwen2.5-coder")
}

/// Check if a model is a reasoning-focused model
pub fn is_reasoning_model(model: &str) -> bool {
    model.starts_with("llama3") || model.starts_with("gemma")
}

/// Check if a model is an embedding model
pub fn is_embedding_model(model: &str) -> bool {
    model.contains("embed") || model.contains("embedding")
}

/// Get recommended models for different use cases
pub fn get_recommended_models() -> HashMap<&'static str, Vec<&'static str>> {
    let mut recommendations = HashMap::new();

    recommendations.insert("chat", vec!["gemma3:4b"]);
    recommendations.insert("code", vec!["qwen2.5-coder:3b"]);
    recommendations.insert("reasoning", vec!["gemma3:4b"]);
    recommendations.insert("fast", vec!["qwen2.5-coder:3b", "gemma3:4b"]);
    recommendations.insert("quality", vec!["gemma3:4b", "qwen2.5-coder:3b"]);
    recommendations.insert("embeddings", vec!["nomic-embed-text:latest"]);

    recommendations
}
