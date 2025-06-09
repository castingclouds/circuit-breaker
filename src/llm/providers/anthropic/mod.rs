//! Anthropic provider module
//! This module provides Anthropic-specific LLM provider implementation

pub mod client;
pub mod config;
pub mod types;

pub use client::AnthropicClient;
pub use config::{
    AnthropicConfig, 
    get_config_requirements, 
    get_default_config, 
    get_available_models,
    is_claude_model,
    has_parameter_restriction,
    model_supports_capability,
    get_model_cost_info,
    format_system_prompt,
    supports_function_calling
};
pub use types::{
    AnthropicRequest,
    AnthropicResponse,
    AnthropicMessage,
    AnthropicUsage,
    AnthropicContentBlock,
    AnthropicStreamingChunk,
    AnthropicDelta,
    AnthropicError,
    AnthropicErrorDetails
};

/// Create a new Anthropic client with API key
pub fn create_client(api_key: String, base_url: Option<String>) -> AnthropicClient {
    let mut config = AnthropicConfig::default();
    config.api_key = api_key;
    
    if let Some(url) = base_url {
        config.base_url = url;
    }
    
    AnthropicClient::new(config)
}

/// Create an Anthropic client from environment variables
pub fn create_client_from_env() -> Result<AnthropicClient, String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable not found")?;
    
    let base_url = std::env::var("ANTHROPIC_BASE_URL").ok();
    
    Ok(create_client(api_key, base_url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::LLMProviderClient;

    #[test]
    fn test_create_client() {
        let client = create_client("test-key".to_string(), None);
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Anthropic);
    }

    #[test]
    fn test_create_client_with_custom_url() {
        let client = create_client(
            "test-key".to_string(), 
            Some("https://custom.anthropic.com".to_string())
        );
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Anthropic);
    }

    #[test]
    fn test_config_requirements() {
        let requirements = get_config_requirements();
        assert_eq!(requirements.api_key_env_var, "ANTHROPIC_API_KEY");
        assert!(requirements.base_url_env_var.is_some());
    }

    #[test]
    fn test_available_models() {
        let models = get_available_models();
        assert!(!models.is_empty());
        
        // Check for specific models
        assert!(models.iter().any(|m| m.id == "claude-3-sonnet-20240229"));
        assert!(models.iter().any(|m| m.id == "claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_claude_model_detection() {
        assert!(is_claude_model("claude-3-sonnet-20240229"));
        assert!(is_claude_model("claude-sonnet-4-20250514"));
        assert!(!is_claude_model("gpt-4"));
        assert!(!is_claude_model("gemini-pro"));
    }
}