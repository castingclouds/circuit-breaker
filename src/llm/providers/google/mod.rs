//! Google provider module
//! This module provides Google Gemini-specific LLM provider implementation

pub mod client;
pub mod config;
pub mod types;

pub use client::GoogleClient;
pub use config::{
    get_available_models, get_config_requirements, get_context_window, get_default_config,
    get_default_safety_settings, get_max_output_tokens, get_model_cost_info,
    has_parameter_restriction, is_gemini_model, model_supports_capability,
    supports_function_calling, supports_multimodal, supports_vision, validate_api_key,
    GoogleConfig,
};
pub use types::{
    convert_conversation_history, create_system_content, GoogleCandidate, GoogleContent,
    GoogleError, GoogleErrorDetails, GoogleFunctionDeclaration, GoogleGenerationConfig,
    GoogleModel, GoogleModelsResponse, GooglePart, GooglePromptFeedback, GoogleRequest,
    GoogleResponse, GoogleSafetyRating, GoogleSafetySetting, GoogleStreamingCandidate,
    GoogleStreamingChunk, GoogleTool, GoogleUsageMetadata,
};

/// Create a new Google client with API key
pub fn create_client(api_key: String, base_url: Option<String>) -> GoogleClient {
    let mut config = GoogleConfig::default();
    config.api_key = api_key;

    if let Some(url) = base_url {
        config.base_url = url;
    }

    GoogleClient::new(config)
}

/// Create a Google client from environment variables
pub fn create_client_from_env() -> Result<GoogleClient, String> {
    let api_key = std::env::var("GOOGLE_API_KEY")
        .map_err(|_| "GOOGLE_API_KEY environment variable not found")?;

    let base_url = std::env::var("GOOGLE_BASE_URL").ok();

    Ok(create_client(api_key, base_url))
}

/// Validate Google API key format
pub fn validate_google_api_key(api_key: &str) -> bool {
    validate_api_key(api_key)
}

/// Get model information by ID
pub fn get_model_info(model_id: &str) -> Option<crate::llm::traits::ModelInfo> {
    get_available_models()
        .into_iter()
        .find(|model| model.id == model_id)
}

/// List all supported model IDs
pub fn list_model_ids() -> Vec<String> {
    get_available_models()
        .into_iter()
        .map(|model| model.id)
        .collect()
}

/// Check if a model supports streaming
pub fn model_supports_streaming(model: &str) -> bool {
    get_available_models()
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.supports_streaming)
        .unwrap_or(false)
}

/// Get recommended models for different use cases
pub fn get_recommended_models() -> std::collections::HashMap<&'static str, Vec<String>> {
    let mut recommendations = std::collections::HashMap::new();
    let default_model =
        std::env::var("GOOGLE_DEFAULT_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());

    // Only recommend the default model from environment
    recommendations.insert("general", vec![default_model.clone()]);
    recommendations.insert("vision", vec![default_model.clone()]);
    recommendations.insert("large_context", vec![default_model.clone()]);
    recommendations.insert("cost_effective", vec![default_model.clone()]);
    recommendations.insert("multimodal", vec![default_model.clone()]);
    recommendations.insert("experimental", vec![default_model]);

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::LLMProviderClient;

    #[test]
    fn test_create_client() {
        let client = create_client("test-key".to_string(), None);
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Google);
    }

    #[test]
    fn test_create_client_with_custom_url() {
        let client = create_client(
            "test-key".to_string(),
            Some("https://custom.googleapis.com/v1beta".to_string()),
        );
        assert_eq!(client.provider_type(), crate::llm::LLMProviderType::Google);
    }

    #[test]
    fn test_config_requirements() {
        let requirements = get_config_requirements();
        assert_eq!(requirements.api_key_env_var, "GOOGLE_API_KEY");
        assert!(requirements.base_url_env_var.is_some());
    }

    #[test]
    fn test_available_models() {
        let models = get_available_models();
        assert!(!models.is_empty());

        // Should only contain one model (the default from environment)
        assert_eq!(models.len(), 1);

        // Check that the model matches the default
        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());
        assert!(models.iter().any(|m| m.id == default_model));
    }

    #[test]
    fn test_gemini_model_detection() {
        assert!(is_gemini_model("gemini-pro"));
        assert!(is_gemini_model("gemini-1.5-flash"));
        assert!(is_gemini_model("gemini-2.5-flash-preview-05-20"));
        assert!(!is_gemini_model("gpt-4"));
        assert!(!is_gemini_model("claude-3"));
    }

    #[test]
    fn test_api_key_validation() {
        // Valid Google API key format
        assert!(validate_google_api_key(
            "AIzaSyDZ65OZIFeraf5qNrf-1vRf3qL54UJMgqU"
        ));

        // Invalid formats
        assert!(!validate_google_api_key("invalid-key"));
        assert!(!validate_google_api_key("sk-1234567890")); // OpenAI format
        assert!(!validate_google_api_key("AIza")); // Too short
    }

    #[test]
    fn test_model_capabilities() {
        assert!(supports_vision("gemini-pro-vision"));
        assert!(supports_multimodal("gemini-1.5-pro"));
        assert!(supports_function_calling("gemini-pro"));
        assert!(!supports_vision("gemini-pro")); // Regular Gemini Pro doesn't support vision
    }

    #[test]
    fn test_context_windows() {
        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());

        // Test context window for the default model
        let context_window = get_context_window(&default_model);
        assert!(context_window > 0);

        // Test fallback for unknown models
        assert_eq!(get_context_window("unknown-model"), 32768); // Default fallback
    }

    #[test]
    fn test_model_recommendations() {
        let recommendations = get_recommended_models();
        assert!(recommendations.contains_key("general"));
        assert!(recommendations.contains_key("vision"));
        assert!(recommendations.contains_key("large_context"));
        assert!(recommendations.contains_key("cost_effective"));

        // Check that recommendations contain the default model
        let general_models = recommendations.get("general").unwrap();
        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());
        assert!(general_models.contains(&default_model));
    }

    #[test]
    fn test_list_model_ids() {
        let model_ids = list_model_ids();
        assert!(!model_ids.is_empty());

        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());
        assert!(model_ids.contains(&default_model));
    }

    #[test]
    fn test_model_info_lookup() {
        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());

        let model_info = get_model_info(&default_model);
        assert!(model_info.is_some());

        let info = model_info.unwrap();
        assert_eq!(info.id, default_model);
        assert_eq!(info.provider, crate::llm::LLMProviderType::Google);

        // Test non-existent model
        let non_existent = get_model_info("non-existent-model");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_streaming_support() {
        let default_model = std::env::var("GOOGLE_DEFAULT_MODEL")
            .unwrap_or_else(|_| "gemini-1.5-flash".to_string());

        assert!(model_supports_streaming(&default_model));
        assert!(!model_supports_streaming("non-existent-model"));
    }
}
