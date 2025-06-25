//! LLM module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for interacting with Large Language Models.

use crate::{schema::QueryBuilder, types::*, Client, Result};
use serde::{Deserialize, Serialize};

/// Common LLM models used across providers
pub mod COMMON_MODELS {
    pub const GPT_3_5_TURBO: &str = "gpt-3.5-turbo";
    pub const GPT_4: &str = "o4-mini-2025-04-16";
    pub const GPT_4O_MINI: &str = "o4-mini-2025-04-16";
    pub const GPT_4_TURBO: &str = "gpt-4-turbo-preview";
    pub const CLAUDE_3_HAIKU: &str = "claude-3-haiku-20240307";
    pub const CLAUDE_3_SONNET: &str = "claude-3-sonnet-20240229";
    pub const CLAUDE_3_OPUS: &str = "claude-3-opus-20240229";
    pub const GEMINI_FLASH: &str = "gemini-1.5-flash";
    pub const GEMINI_PRO: &str = "gemini-1.5-pro";
}

/// Client for LLM operations
#[derive(Debug, Clone)]
pub struct LLMClient {
    client: Client,
}

impl LLMClient {
    /// Create a new LLM client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Make a chat completion request
    pub async fn chat_completion(&self, request: LLMRequest) -> Result<LLMResponse> {
        let mutation = QueryBuilder::mutation_with_params(
            "LlmChatCompletion",
            "llmChatCompletion(input: $input)",
            &[
                "id",
                "model",
                "choices { index message { role content } finishReason }",
                "usage { promptTokens completionTokens totalTokens }",
            ],
            &[("input", "LlmchatCompletionInput!")],
        );

        #[derive(Serialize)]
        struct Variables {
            input: LlmchatCompletionInput,
        }

        #[derive(Serialize)]
        struct LlmchatCompletionInput {
            model: String,
            messages: Vec<ChatMessage>,
            temperature: Option<f32>,
            #[serde(rename = "maxTokens")]
            max_tokens: Option<u32>,
            stream: Option<bool>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmChatCompletion")]
            llm_chat_completion: LLMResponse,
        }

        let response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    input: LlmchatCompletionInput {
                        model: request.model,
                        messages: request.messages,
                        temperature: request.temperature,
                        max_tokens: request.max_tokens,
                        stream: request.stream,
                    },
                },
            )
            .await?;

        Ok(response.llm_chat_completion)
    }

    /// Get available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let query = QueryBuilder::query(
            "ListModels",
            "llmModels",
            &["name", "provider", "available"],
        );

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmModels")]
            llm_models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
            provider: String,
            available: bool,
        }

        let response: Response = self.client.graphql(&query, ()).await?;

        Ok(response
            .llm_models
            .into_iter()
            .filter(|model| model.available)
            .map(|model| model.name)
            .collect())
    }

    /// Simple chat method with just model and message
    pub async fn chat(&self, model: &str, message: &str) -> Result<String> {
        let request = LLMRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: message.to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: Some(false),
        };

        let response = self.chat_completion(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(crate::Error::Parse {
                message: "No response choices available".to_string(),
            })
        }
    }

    /// Chat with system prompt
    pub async fn chat_with_system(
        &self,
        model: &str,
        system: &str,
        message: &str,
    ) -> Result<String> {
        let request = LLMRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: system.to_string(),
                    name: None,
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: message.to_string(),
                    name: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: Some(false),
        };

        let response = self.chat_completion(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(crate::Error::Parse {
                message: "No response choices available".to_string(),
            })
        }
    }
}

/// Builder for creating chat requests
pub struct ChatBuilder {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
}

impl ChatBuilder {
    /// Create a new chat builder
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: None,
        }
    }

    /// Set system prompt
    pub fn set_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::System,
            content: prompt.into(),
            name: None,
        });
        self
    }

    /// Add user message
    pub fn add_user_message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: message.into(),
            name: None,
        });
        self
    }

    /// Add assistant message
    pub fn add_assistant_message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: message.into(),
            name: None,
        });
        self
    }

    /// Set temperature
    pub fn set_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn set_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set streaming
    pub fn set_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Build the chat request
    pub fn build(self) -> LLMRequest {
        LLMRequest {
            model: self.model,
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
        }
    }

    /// Execute the chat request
    pub async fn execute(self, client: &LLMClient) -> Result<LLMResponse> {
        client.chat_completion(self.build()).await
    }
}

/// Convenience function to create a chat builder
pub fn create_chat(model: impl Into<String>) -> ChatBuilder {
    ChatBuilder::new(model)
}

/// Helper to create a simple chat request
pub fn chat_request(model: &str, message: &str) -> LLMRequest {
    LLMRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        stream: Some(false),
    }
}
