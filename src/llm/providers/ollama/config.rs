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
            default_model: "llama3.2:3b".to_string(), // Generic fallback, will be overridden by env
            timeout_seconds: 60,                      // Longer timeout for local inference
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
        default_model: "llama3.2:3b".to_string(), // Generic fallback, will be overridden by env
        models: Vec::new(),                       // Models will be fetched dynamically from Ollama
        settings: HashMap::new(),
        enabled: true,
        priority: 3, // Lower priority than cloud providers by default
    }
}

/// Get fallback models when Ollama is not available
/// These are common models that might be available, but actual models should be fetched dynamically
pub fn get_fallback_models() -> Vec<ModelInfo> {
    // Return empty vec - models should be fetched dynamically from Ollama
    // This is only used as a last resort when Ollama is completely unavailable
    Vec::new()
}

/// Check if a model supports a specific capability
/// Note: This is a heuristic based on model name patterns since models are dynamic
pub fn model_supports_capability(model: &str, capability: &ModelCapability) -> bool {
    use ModelCapability::*;

    match capability {
        CodeGeneration => is_code_model(model),
        Embedding => is_embedding_model(model),
        TextGeneration | ConversationalAI => true, // Most models support these
        Reasoning => is_reasoning_model(model) || is_code_model(model),
        _ => false, // Conservative default for unknown capabilities
    }
}

/// Get model information by ID
/// Note: This creates a generic ModelInfo since models are fetched dynamically
pub fn get_model_info(model: &str) -> Option<ModelInfo> {
    // Create a generic ModelInfo for any model name
    // Actual capabilities should be determined by the Ollama client
    Some(create_generic_model_info(model))
}

/// Create a generic ModelInfo for a given model name
fn create_generic_model_info(model_id: &str) -> ModelInfo {
    let capabilities = determine_model_capabilities_from_name(model_id);
    let (context_window, max_output_tokens) = estimate_model_size_from_name(model_id);

    ModelInfo {
        id: model_id.to_string(),
        name: format!("Ollama: {}", model_id),
        provider: LLMProviderType::Ollama,
        context_window,
        max_output_tokens,
        supports_streaming: true,
        supports_function_calling: false, // Most local models don't support this yet
        cost_per_input_token: 0.0,        // Local inference is free
        cost_per_output_token: 0.0,
        capabilities,
        parameter_restrictions: HashMap::new(),
    }
}

/// Determine model capabilities based on name patterns
fn determine_model_capabilities_from_name(name: &str) -> Vec<crate::llm::traits::ModelCapability> {
    use crate::llm::traits::ModelCapability;

    let mut capabilities = vec![
        ModelCapability::TextGeneration,
        ModelCapability::ConversationalAI,
    ];

    // Code generation models
    if name.contains("code") || name.contains("phi") || name.contains("granite") {
        capabilities.push(ModelCapability::CodeGeneration);
    }

    // Reasoning models
    if name.contains("llama") || name.contains("gemma") || name.contains("mistral") {
        capabilities.push(ModelCapability::Reasoning);
    }

    // Embedding models
    if name.contains("embed") {
        capabilities.push(ModelCapability::Embedding);
        // Embedding models typically don't do text generation
        capabilities.retain(|cap| {
            !matches!(
                cap,
                ModelCapability::TextGeneration | ModelCapability::ConversationalAI
            )
        });
    }

    capabilities
}

/// Estimate model capabilities from name patterns
fn estimate_model_size_from_name(name: &str) -> (u32, u32) {
    // Extract parameter size hints from model name
    if name.contains("3b") || name.contains("3B") {
        (8192, 4096)
    } else if name.contains("7b") || name.contains("7B") {
        (16384, 8192)
    } else if name.contains("13b") || name.contains("13B") {
        (32768, 16384)
    } else if name.contains("70b") || name.contains("70B") {
        (65536, 32768)
    } else {
        (8192, 4096) // Conservative default
    }
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

/// Get recommended model patterns for different use cases
/// Note: These are patterns/prefixes since actual models are dynamic
pub fn get_recommended_model_patterns() -> HashMap<&'static str, Vec<&'static str>> {
    let mut recommendations = HashMap::new();

    recommendations.insert("chat", vec!["llama", "gemma", "mistral"]);
    recommendations.insert("code", vec!["coder", "codellama", "code", "granite"]);
    recommendations.insert("reasoning", vec!["llama", "gemma", "mistral"]);
    recommendations.insert("fast", vec!["gemma", "phi"]);
    recommendations.insert("quality", vec!["llama", "mistral"]);
    recommendations.insert("embeddings", vec!["embed", "nomic"]);

    recommendations
}
