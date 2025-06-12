//! vLLM provider configuration
//! This module contains configuration structures and defaults specific to vLLM

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::llm::{LLMProviderType, traits::{
    ModelInfo, ProviderConfig, ProviderConfigRequirements,
    RateLimitInfo, ModelCapability
}};

/// vLLM-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMConfig {
    /// Base URL for vLLM API (typically http://localhost:8000)
    pub base_url: String,
    /// Optional API key (vLLM can be configured with authentication)
    pub api_key: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,
}

impl Default for VLLMConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            api_key: None,
            default_model: "microsoft/DialoGPT-medium".to_string(),
            timeout_seconds: 120, // Longer timeout for model loading
            max_retries: 3,
            verify_ssl: true,
            custom_headers: HashMap::new(),
        }
    }
}

/// Get vLLM provider configuration requirements
pub fn get_config_requirements() -> ProviderConfigRequirements {
    let parameter_restrictions = HashMap::new();

    ProviderConfigRequirements {
        api_key_env_var: "VLLM_API_KEY".to_string(),
        base_url_env_var: Some("VLLM_BASE_URL".to_string()),
        auth_methods: vec![], // vLLM optionally supports API keys
        rate_limits: Some(RateLimitInfo {
            requests_per_minute: None, // No hard limits for local instance
            tokens_per_minute: None,
            requests_per_day: None,
            concurrent_requests: Some(256), // Hardware dependent
        }),
        parameter_restrictions,
    }
}

/// Get default vLLM provider configuration
pub fn get_default_config() -> ProviderConfig {
    ProviderConfig {
        provider_type: LLMProviderType::VLLM,
        base_url: "http://localhost:8000".to_string(),
        default_model: "microsoft/DialoGPT-medium".to_string(),
        models: get_default_models(),
        settings: HashMap::new(),
        enabled: true,
        priority: 2, // Higher priority than Ollama due to better performance
    }
}

/// Get default vLLM models (commonly used models)
pub fn get_default_models() -> Vec<ModelInfo> {
    vec![
        // Lightweight models for development/testing
        ModelInfo {
            id: "microsoft/DialoGPT-medium".to_string(),
            name: "DialoGPT Medium".to_string(),
            provider: LLMProviderType::VLLM,
            context_window: 1024,
            max_output_tokens: 512,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.0, // Local inference is free
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // Code generation models
        ModelInfo {
            id: "codellama/CodeLlama-7b-Instruct-hf".to_string(),
            name: "Code Llama 7B Instruct".to_string(),
            provider: LLMProviderType::VLLM,
            context_window: 4096,
            max_output_tokens: 2048,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::CodeGeneration,
                ModelCapability::TextGeneration,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // General purpose models
        ModelInfo {
            id: "meta-llama/Llama-2-7b-chat-hf".to_string(),
            name: "Llama 2 7B Chat".to_string(),
            provider: LLMProviderType::VLLM,
            context_window: 4096,
            max_output_tokens: 2048,
            supports_streaming: true,
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::ConversationalAI,
                ModelCapability::ReasoningChain,
            ],
            parameter_restrictions: HashMap::new(),
        },
        // Embedding models
        ModelInfo {
            id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            name: "All MiniLM L6 v2".to_string(),
            provider: LLMProviderType::VLLM,
            context_window: 512,
            max_output_tokens: 0, // Embedding models don't generate text
            supports_streaming: false,
            supports_function_calling: false,
            cost_per_input_token: 0.0,
            cost_per_output_token: 0.0,
            capabilities: vec![
                ModelCapability::Embedding,
            ],
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

/// Check if a model is primarily for code generation
pub fn is_code_model(model: &str) -> bool {
    model.to_lowercase().contains("code")
}

/// Check if a model is for embeddings
pub fn is_embedding_model(model: &str) -> bool {
    model.to_lowercase().contains("embedding") || 
    model.to_lowercase().contains("sentence-transformers")
}

/// Get recommended models for different use cases
pub fn get_recommended_models() -> HashMap<&'static str, Vec<&'static str>> {
    let mut recommendations = HashMap::new();
    
    recommendations.insert("chat", vec![
        "meta-llama/Llama-2-7b-chat-hf",
        "microsoft/DialoGPT-medium",
    ]);
    
    recommendations.insert("code", vec![
        "codellama/CodeLlama-7b-Instruct-hf",
    ]);
    
    recommendations.insert("embeddings", vec![
        "sentence-transformers/all-MiniLM-L6-v2",
    ]);
    
    recommendations.insert("fast", vec![
        "microsoft/DialoGPT-medium",
    ]);
    
    recommendations.insert("quality", vec![
        "meta-llama/Llama-2-7b-chat-hf",
        "codellama/CodeLlama-7b-Instruct-hf",
    ]);
    
    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VLLMConfig::default();
        assert_eq!(config.base_url, "http://localhost:8000");
        assert_eq!(config.default_model, "microsoft/DialoGPT-medium");
        assert!(config.verify_ssl);
    }

    #[test]
    fn test_model_capabilities() {
        assert!(is_code_model("codellama/CodeLlama-7b-Instruct-hf"));
        assert!(!is_code_model("meta-llama/Llama-2-7b-chat-hf"));
        
        assert!(is_embedding_model("sentence-transformers/all-MiniLM-L6-v2"));
        assert!(!is_embedding_model("microsoft/DialoGPT-medium"));
    }

    #[test]
    fn test_get_model_info() {
        let model_info = get_model_info("microsoft/DialoGPT-medium");
        assert!(model_info.is_some());
        
        let info = model_info.unwrap();
        assert_eq!(info.provider, LLMProviderType::VLLM);
        assert!(info.supports_streaming);
    }

    #[test]
    fn test_recommended_models() {
        let recommendations = get_recommended_models();
        assert!(recommendations.contains_key("chat"));
        assert!(recommendations.contains_key("code"));
        assert!(recommendations.contains_key("embeddings"));
    }
}