//! OpenAI provider module
//! This module provides OpenAI-specific LLM provider implementation

pub mod client;
pub mod config;
pub mod types;

pub use client::OpenAIClient;
pub use config::{
    OpenAIConfig, 
    get_config_requirements, 
    get_default_config, 
    get_available_models,
    is_o4_model,
    has_parameter_restriction,
    model_supports_capability,
    get_model_cost_info
};
pub use types::{
    OpenAIRequest,
    OpenAIResponse,
    OpenAIChatMessage,
    OpenAIChoice,
    OpenAIUsage,
    OpenAIStreamingChunk,
    OpenAIStreamingChoice,
    OpenAIError,
    OpenAIErrorDetails,
    OpenAIModel,
    OpenAIModelsResponse,
    ResponseFormat,
    Tool,
    Function,
    ToolChoice,
    ToolCall,
    FunctionCall,
    FunctionChoice
};

/// Create a new OpenAI client with API key
pub fn create_client(api_key: String, base_url: Option<String>) -> OpenAIClient {
    let mut config = OpenAIConfig::default();
    config.api_key = api_key;
    
    if let Some(url) = base_url {
        config.base_url = url;
    }
    
    OpenAIClient::new(config)
}

/// Create an OpenAI client from environment variables
pub fn create_client_from_env() -> Result<OpenAIClient, String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY environment variable not found")?;
    
    let base_url = std::env::var("OPENAI_BASE_URL").ok();
    
    Ok(create_client(api_key, base_url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::LLMProviderClient;

    #[test]
    fn test_create_client() {
        let client = create_client("test-key".to_string(), None);
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::OpenAI);
    }

    #[test]
    fn test_create_client_with_custom_url() {
        let client = create_client(
            "test-key".to_string(), 
            Some("https://custom.openai.com/v1".to_string())
        );
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::OpenAI);
    }

    #[test]
    fn test_config_requirements() {
        let requirements = get_config_requirements();
        assert_eq!(requirements.api_key_env_var, "OPENAI_API_KEY");
        assert!(requirements.base_url_env_var.is_some());
    }

    #[test]
    fn test_available_models() {
        let models = get_available_models();
        assert!(!models.is_empty());
        
        // Check for specific models
        assert!(models.iter().any(|m| m.id == "gpt-4"));
        assert!(models.iter().any(|m| m.id == "o4-mini-2025-04-16"));
    }

    #[test]
    fn test_o4_model_detection() {
        assert!(is_o4_model("o4-mini-2025-04-16"));
        assert!(is_o4_model("o4-2025-04-16"));
        assert!(!is_o4_model("gpt-4"));
        assert!(!is_o4_model("claude-3"));
    }
}