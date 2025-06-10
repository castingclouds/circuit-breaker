//! Ollama provider module
//! This module provides Ollama-specific LLM provider implementation

pub mod client;
pub mod config;
pub mod types;

pub use client::OllamaClient;
pub use config::{
    OllamaConfig, 
    get_config_requirements, 
    get_default_config, 
    get_default_models,
    is_code_model,
    is_reasoning_model,
    is_embedding_model,
    model_supports_capability,
    get_model_info,
    get_recommended_models
};
pub use types::{
    OllamaRequest,
    OllamaResponse,
    OllamaChatMessage,
    OllamaStreamingChunk,
    OllamaOptions,
    OllamaModelInfo,
    OllamaModelsResponse,
    OllamaError,
    OllamaGenerateRequest,
    OllamaGenerateResponse,
    OllamaModelDetails,
    OllamaHealthResponse,
    OllamaEmbeddingsRequest,
    OllamaEmbeddingsResponse,
    OllamaBatchEmbeddingsRequest,
    OllamaBatchEmbeddingsResponse
};

/// Create a new Ollama client with base URL
pub fn create_client(base_url: String) -> OllamaClient {
    let mut config = OllamaConfig::default();
    config.base_url = base_url;
    OllamaClient::new(config)
}

/// Create a new Ollama client with custom configuration
pub fn create_client_with_config(config: OllamaConfig) -> OllamaClient {
    OllamaClient::new(config)
}

/// Create an Ollama client from environment variables
pub fn create_client_from_env() -> Result<OllamaClient, String> {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    let mut config = OllamaConfig::default();
    config.base_url = base_url;
    
    // Optional: Set default model from environment
    if let Ok(default_model) = std::env::var("OLLAMA_DEFAULT_MODEL") {
        config.default_model = default_model;
    }
    
    // Optional: Set keep_alive from environment
    if let Ok(keep_alive) = std::env::var("OLLAMA_KEEP_ALIVE") {
        config.keep_alive = keep_alive;
    }
    
    // Optional: Disable SSL verification for self-signed certificates
    if let Ok(verify_ssl) = std::env::var("OLLAMA_VERIFY_SSL") {
        config.verify_ssl = verify_ssl.parse().unwrap_or(true);
    }
    
    Ok(OllamaClient::new(config))
}

/// Check if Ollama is available at the given base URL
pub async fn check_availability(base_url: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", base_url);
    
    match client.get(&url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Get available models from Ollama instance
pub async fn fetch_available_models(base_url: &str) -> Result<Vec<crate::llm::traits::ModelInfo>, String> {
    let client = create_client(base_url.to_string());
    client.fetch_available_models().await
        .map_err(|e| format!("Failed to fetch models: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::LLMProviderClient;

    #[test]
    fn test_create_client() {
        let client = create_client("http://localhost:11434".to_string());
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Ollama);
    }

    #[test]
    fn test_create_client_with_custom_config() {
        let mut config = OllamaConfig::default();
        config.base_url = "http://custom:8080".to_string();
        config.default_model = "custom-model".to_string();
        
        let client = create_client_with_config(config);
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Ollama);
    }

    #[test]
    fn test_create_client_from_env() {
        // Test with default values when env vars are not set
        let client = create_client_from_env().unwrap();
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Ollama);
    }

    #[test]
    fn test_config_requirements() {
        let requirements = get_config_requirements();
        assert_eq!(requirements.api_key_env_var, "OLLAMA_API_KEY");
        assert!(requirements.base_url_env_var.is_some());
        assert_eq!(requirements.base_url_env_var.unwrap(), "OLLAMA_BASE_URL");
    }

    #[test]
    fn test_default_models() {
        let models = get_default_models();
        assert!(!models.is_empty());
        
        // Check for specific models
        assert!(models.iter().any(|m| m.id == "qwen2.5-coder:3b"));
        assert!(models.iter().any(|m| m.id == "gemma3:4b"));
        assert!(models.iter().any(|m| m.id == "nomic-embed-text:latest"));
    }

    #[test]
    fn test_model_classification() {
        assert!(is_code_model("qwen2.5-coder:3b"));
        assert!(is_code_model("codellama"));
        assert!(is_code_model("phi"));
        assert!(!is_code_model("llama2"));
        
        assert!(is_reasoning_model("llama3"));
        assert!(is_reasoning_model("gemma3:4b"));
        assert!(!is_reasoning_model("codellama"));
        
        assert!(is_embedding_model("nomic-embed-text:latest"));
        assert!(!is_embedding_model("qwen2.5-coder:3b"));
    }

    #[test]
    fn test_recommended_models() {
        let recommendations = get_recommended_models();
        
        assert!(recommendations.contains_key("chat"));
        assert!(recommendations.contains_key("code"));
        assert!(recommendations.contains_key("reasoning"));
        assert!(recommendations.contains_key("fast"));
        assert!(recommendations.contains_key("quality"));
        
        // Check that recommendations contain expected models
        let chat_models = recommendations.get("chat").unwrap();
        assert!(chat_models.contains(&"gemma3:4b"));
        
        let code_models = recommendations.get("code").unwrap();
        assert!(code_models.contains(&"qwen2.5-coder:3b"));
        
        let embedding_models = recommendations.get("embeddings").unwrap();
        assert!(embedding_models.contains(&"nomic-embed-text:latest"));
    }

    #[tokio::test]
    async fn test_check_availability() {
        // This test would require a running Ollama instance
        // In a real test environment, you might use a mock server
        let available = check_availability("http://localhost:11434").await;
        // Don't assert the result since it depends on whether Ollama is running
        println!("Ollama availability: {}", available);
    }
}