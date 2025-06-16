//! vLLM provider module
//! This module provides vLLM-specific LLM provider implementation for OpenAI-compatible API

pub mod client;
pub mod config;
pub mod types;

pub use client::VLLMClient;
pub use config::{
    get_config_requirements, get_default_config, get_default_models, get_model_info,
    get_recommended_models, is_code_model, is_embedding_model, model_supports_capability,
    VLLMConfig,
};
pub use types::{
    VLLMChatMessage, VLLMChoice, VLLMEmbedding, VLLMEmbeddingsRequest, VLLMEmbeddingsResponse,
    VLLMEmbeddingsUsage, VLLMError, VLLMErrorDetails, VLLMHealthResponse, VLLMModel,
    VLLMModelsResponse, VLLMRequest, VLLMResponse, VLLMServerInfo, VLLMStreamingChoice,
    VLLMStreamingChunk, VLLMUsage,
};

/// Create a new vLLM client with base URL
pub fn create_client(base_url: String) -> VLLMClient {
    let mut config = VLLMConfig::default();
    config.base_url = base_url;
    VLLMClient::new(config)
}

/// Create a new vLLM client with custom configuration
pub fn create_client_with_config(config: VLLMConfig) -> VLLMClient {
    VLLMClient::new(config)
}

/// Create a vLLM client from environment variables
pub fn create_client_from_env() -> Result<VLLMClient, String> {
    let base_url =
        std::env::var("VLLM_BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let mut config = VLLMConfig::default();
    config.base_url = base_url;

    // Optional: Set API key from environment (vLLM can be configured with auth)
    if let Ok(api_key) = std::env::var("VLLM_API_KEY") {
        config.api_key = Some(api_key);
    }

    // Optional: Set default model from environment
    if let Ok(default_model) = std::env::var("VLLM_DEFAULT_MODEL") {
        config.default_model = default_model;
    }

    // Optional: Disable SSL verification for self-signed certificates
    if let Ok(verify_ssl) = std::env::var("VLLM_VERIFY_SSL") {
        config.verify_ssl = verify_ssl.parse().unwrap_or(true);
    }

    Ok(VLLMClient::new(config))
}

/// Check if vLLM is available at the given base URL
pub async fn check_availability(base_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let url = format!("{}/v1/models", base_url);

    match client.get(&url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Get available models from vLLM instance
pub async fn fetch_available_models(
    base_url: &str,
) -> Result<Vec<crate::llm::traits::ModelInfo>, String> {
    let client = create_client(base_url.to_string());
    client
        .fetch_available_models()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::LLMProviderClient;

    #[test]
    fn test_create_client() {
        let client = create_client("http://localhost:8000".to_string());
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::VLLM);
    }

    #[test]
    fn test_create_client_with_custom_config() {
        let mut config = VLLMConfig::default();
        config.base_url = "http://custom:8080".to_string();
        config.default_model = "meta-llama/Llama-2-7b-chat-hf".to_string();

        let client = create_client_with_config(config);
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::VLLM);
    }

    #[test]
    fn test_create_client_from_env() {
        // Test with default values when env vars are not set
        let client = create_client_from_env().unwrap();
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::VLLM);
    }

    #[test]
    fn test_config_requirements() {
        let requirements = get_config_requirements();
        assert_eq!(requirements.api_key_env_var, "VLLM_API_KEY");
        assert!(requirements.base_url_env_var.is_some());
        assert_eq!(requirements.base_url_env_var.unwrap(), "VLLM_BASE_URL");
    }

    #[test]
    fn test_default_models() {
        let models = get_default_models();
        assert!(!models.is_empty());

        // Check for specific models
        assert!(models.iter().any(|m| m.id == "microsoft/DialoGPT-medium"));
        assert!(models
            .iter()
            .any(|m| m.id == "meta-llama/Llama-2-7b-chat-hf"));
        assert!(models
            .iter()
            .any(|m| m.id == "codellama/CodeLlama-7b-Instruct-hf"));
    }

    #[test]
    fn test_model_classification() {
        assert!(is_code_model("codellama/CodeLlama-7b-Instruct-hf"));
        assert!(!is_code_model("meta-llama/Llama-2-7b-chat-hf"));

        assert!(is_embedding_model("sentence-transformers/all-MiniLM-L6-v2"));
        assert!(!is_embedding_model("meta-llama/Llama-2-7b-chat-hf"));
    }

    #[test]
    fn test_recommended_models() {
        let recommendations = get_recommended_models();

        assert!(recommendations.contains_key("chat"));
        assert!(recommendations.contains_key("code"));
        assert!(recommendations.contains_key("embeddings"));

        // Check that recommendations contain expected models
        let chat_models = recommendations.get("chat").unwrap();
        assert!(chat_models.contains(&"microsoft/DialoGPT-medium"));

        let code_models = recommendations.get("code").unwrap();
        assert!(code_models.contains(&"codellama/CodeLlama-7b-Instruct-hf"));

        let embedding_models = recommendations.get("embeddings").unwrap();
        assert!(embedding_models.contains(&"sentence-transformers/all-MiniLM-L6-v2"));
    }

    #[tokio::test]
    async fn test_check_availability() {
        // This test would require a running vLLM instance
        // In a real test environment, you might use a mock server
        let available = check_availability("http://localhost:8000").await;
        // Don't assert the result since it depends on whether vLLM is running
        println!("vLLM availability: {}", available);
    }
}
