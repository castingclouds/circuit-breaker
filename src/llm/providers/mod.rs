//! LLM Providers Module
//! 
//! This module contains implementations for different LLM providers, organized
//! by provider with each having their own subdirectory containing:
//! - client.rs: Provider-specific client implementation
//! - config.rs: Provider-specific configuration and model definitions
//! - types.rs: Provider-specific request/response types
//! - mod.rs: Module exports

pub mod openai;
pub mod anthropic;
pub mod google;
pub mod ollama;

use std::collections::HashMap;
use crate::llm::{LLMProviderType, traits::{LLMProviderClient, ProviderFactory, ProviderConfig}};

// Re-export provider clients for convenience
pub use openai::OpenAIClient;
pub use anthropic::AnthropicClient;
pub use google::GoogleClient;
pub use ollama::OllamaClient;

/// Provider factory registry for creating provider clients
pub struct ProviderRegistry {
    factories: HashMap<LLMProviderType, Box<dyn ProviderFactory>>,
    clients: HashMap<LLMProviderType, Box<dyn LLMProviderClient>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    /// Register a provider factory
    pub fn register_factory(&mut self, factory: Box<dyn ProviderFactory>) {
        let provider_type = factory.provider_type();
        self.factories.insert(provider_type, factory);
    }

    /// Create and register a provider client
    pub fn create_provider(&mut self, provider_type: LLMProviderType, config: &ProviderConfig) -> Result<(), String> {
        if let Some(factory) = self.factories.get(&provider_type) {
            let client = factory.create_client(config);
            self.clients.insert(provider_type, client);
            Ok(())
        } else {
            Err(format!("No factory registered for provider: {:?}", provider_type))
        }
    }

    /// Get a provider client
    pub fn get_provider(&self, provider_type: &LLMProviderType) -> Option<&dyn LLMProviderClient> {
        self.clients.get(provider_type).map(|client| client.as_ref())
    }

    /// Get all available provider types
    pub fn get_available_providers(&self) -> Vec<LLMProviderType> {
        self.clients.keys().cloned().collect()
    }

    /// Check if a provider is available
    pub fn is_provider_available(&self, provider_type: &LLMProviderType) -> bool {
        self.clients.contains_key(provider_type)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenAI provider factory
pub struct OpenAIFactory;

impl ProviderFactory for OpenAIFactory {
    fn create_client(&self, config: &ProviderConfig) -> Box<dyn LLMProviderClient> {
        // Extract API key from config settings
        let api_key = config.settings
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut openai_config = openai::OpenAIConfig::default();
        openai_config.api_key = api_key;
        openai_config.base_url = config.base_url.clone();
        openai_config.default_model = config.default_model.clone();

        Box::new(openai::OpenAIClient::new(openai_config))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::OpenAI
    }

    fn default_config(&self) -> ProviderConfig {
        openai::get_default_config()
    }
}

/// Anthropic provider factory
pub struct AnthropicFactory;

impl ProviderFactory for AnthropicFactory {
    fn create_client(&self, config: &ProviderConfig) -> Box<dyn LLMProviderClient> {
        // Extract API key from config settings
        let api_key = config.settings
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut anthropic_config = anthropic::AnthropicConfig::default();
        anthropic_config.api_key = api_key;
        anthropic_config.base_url = config.base_url.clone();
        anthropic_config.default_model = config.default_model.clone();

        Box::new(anthropic::AnthropicClient::new(anthropic_config))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Anthropic
    }

    fn default_config(&self) -> ProviderConfig {
        anthropic::get_default_config()
    }
}

/// Google provider factory
pub struct GoogleFactory;

impl ProviderFactory for GoogleFactory {
    fn create_client(&self, config: &ProviderConfig) -> Box<dyn LLMProviderClient> {
        // Extract API key from config settings
        let api_key = config.settings
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut google_config = google::GoogleConfig::default();
        google_config.api_key = api_key;
        google_config.base_url = config.base_url.clone();
        google_config.default_model = config.default_model.clone();

        Box::new(google::GoogleClient::new(google_config))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Google
    }

    fn default_config(&self) -> ProviderConfig {
        google::get_default_config()
    }
}

/// Ollama provider factory
pub struct OllamaFactory;

impl ProviderFactory for OllamaFactory {
    fn create_client(&self, config: &ProviderConfig) -> Box<dyn LLMProviderClient> {
        let mut ollama_config = ollama::OllamaConfig::default();
        ollama_config.base_url = config.base_url.clone();
        ollama_config.default_model = config.default_model.clone();

        // Extract optional settings from config
        if let Some(keep_alive) = config.settings.get("keep_alive").and_then(|v| v.as_str()) {
            ollama_config.keep_alive = keep_alive.to_string();
        }

        if let Some(verify_ssl) = config.settings.get("verify_ssl").and_then(|v| v.as_bool()) {
            ollama_config.verify_ssl = verify_ssl;
        }

        if let Some(timeout) = config.settings.get("timeout_seconds").and_then(|v| v.as_u64()) {
            ollama_config.timeout_seconds = timeout;
        }

        Box::new(ollama::OllamaClient::new(ollama_config))
    }

    fn provider_type(&self) -> LLMProviderType {
        LLMProviderType::Ollama
    }

    fn default_config(&self) -> ProviderConfig {
        ollama::get_default_config()
    }
}

/// Create a provider registry with all available providers
pub fn create_default_registry() -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();
    
    // Register OpenAI factory
    registry.register_factory(Box::new(OpenAIFactory));
    
    // Register Anthropic factory
    registry.register_factory(Box::new(AnthropicFactory));
    
    // Register Google factory
    registry.register_factory(Box::new(GoogleFactory));
    
    // Register Ollama factory
    registry.register_factory(Box::new(OllamaFactory));
    
    registry
}

/// Legacy compatibility function - creates provider clients directly
/// TODO: Remove this once router.rs is updated to use the new registry
pub fn create_provider_client(provider_type: LLMProviderType, base_url: Option<String>) -> Box<dyn LLMProviderClient> {
    match provider_type {
        LLMProviderType::OpenAI => {
            let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            Box::new(openai::create_client(api_key, base_url))
        },
        LLMProviderType::Anthropic => {
            let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
            Box::new(anthropic::create_client(api_key, base_url))
        },
        LLMProviderType::Google => {
            let api_key = std::env::var("GOOGLE_API_KEY").unwrap_or_default();
            Box::new(google::create_client(api_key, base_url))
        },
        LLMProviderType::Ollama => {
            let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
            Box::new(ollama::create_client(base_url))
        },
        _ => panic!("Provider not yet implemented: {:?}", provider_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let mut registry = ProviderRegistry::new();
        registry.register_factory(Box::new(OpenAIFactory));
        
        assert!(registry.factories.contains_key(&LLMProviderType::OpenAI));
    }

    #[test]
    fn test_create_default_registry() {
        let registry = create_default_registry();
        assert!(!registry.factories.is_empty());
    }

    #[test]
    fn test_openai_factory() {
        let factory = OpenAIFactory;
        assert_eq!(factory.provider_type(), LLMProviderType::OpenAI);
        
        let config = factory.default_config();
        assert_eq!(config.provider_type, LLMProviderType::OpenAI);
        assert!(!config.models.is_empty());
    }

    #[test]
    fn test_ollama_factory() {
        let factory = OllamaFactory;
        assert_eq!(factory.provider_type(), LLMProviderType::Ollama);
        
        let config = factory.default_config();
        assert_eq!(config.provider_type, LLMProviderType::Ollama);
        assert!(!config.models.is_empty());
    }
}