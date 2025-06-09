//! Google provider-specific types and structures
//! This module contains all the request/response types specific to Google's Gemini API

use serde::{Deserialize, Serialize};
use crate::llm::{ChatMessage, TokenUsage, MessageRole};

/// Google API request structure for chat completions
#[derive(Debug, Clone, Serialize)]
pub struct GoogleRequest {
    pub contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GoogleGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<GoogleSafetySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GoogleTool>>,
}

/// Google content structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleContent {
    #[serde(default)]
    pub parts: Vec<GooglePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Google content part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooglePart {
    pub text: String,
}

/// Google generation configuration
#[derive(Debug, Clone, Serialize)]
pub struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Google safety setting
#[derive(Debug, Clone, Serialize)]
pub struct GoogleSafetySetting {
    pub category: String,
    pub threshold: String,
}

/// Google tool definition
#[derive(Debug, Clone, Serialize)]
pub struct GoogleTool {
    pub function_declarations: Vec<GoogleFunctionDeclaration>,
}

/// Google function declaration
#[derive(Debug, Clone, Serialize)]
pub struct GoogleFunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Google API response structure
#[derive(Debug, Deserialize)]
pub struct GoogleResponse {
    pub candidates: Vec<GoogleCandidate>,
    #[serde(rename = "usageMetadata", skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GoogleUsageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<GooglePromptFeedback>,
}

/// Google response candidate
#[derive(Debug, Deserialize)]
pub struct GoogleCandidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<GoogleContent>,
    #[serde(rename = "finishReason", skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<GoogleSafetyRating>>,
}

/// Google usage metadata
#[derive(Debug, Deserialize)]
pub struct GoogleUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: u32,
}

/// Google prompt feedback
#[derive(Debug, Deserialize)]
pub struct GooglePromptFeedback {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<GoogleSafetyRating>>,
}

/// Google safety rating
#[derive(Debug, Deserialize)]
pub struct GoogleSafetyRating {
    pub category: String,
    pub probability: String,
}

/// Google streaming response chunk
#[derive(Debug, Deserialize)]
pub struct GoogleStreamingChunk {
    pub candidates: Vec<GoogleStreamingCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GoogleUsageMetadata>,
}

/// Google streaming candidate
#[derive(Debug, Deserialize)]
pub struct GoogleStreamingCandidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<GoogleContent>,
    #[serde(rename = "finishReason", skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

/// Google error response
#[derive(Debug, Deserialize)]
pub struct GoogleError {
    pub error: GoogleErrorDetails,
}

/// Google error details
#[derive(Debug, Deserialize)]
pub struct GoogleErrorDetails {
    pub code: u32,
    pub message: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<serde_json::Value>>,
}

/// Google models list response
#[derive(Debug, Deserialize)]
pub struct GoogleModelsResponse {
    pub models: Vec<GoogleModel>,
}

/// Google model information
#[derive(Debug, Deserialize)]
pub struct GoogleModel {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: String,
    #[serde(rename = "inputTokenLimit")]
    pub input_token_limit: u32,
    #[serde(rename = "outputTokenLimit")]
    pub output_token_limit: u32,
    #[serde(rename = "supportedGenerationMethods")]
    pub supported_generation_methods: Vec<String>,
}

impl From<&ChatMessage> for GoogleContent {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            parts: vec![GooglePart {
                text: msg.content.clone(),
            }],
            role: Some(match msg.role {
                MessageRole::System => "user".to_string(), // Google doesn't have system role
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "model".to_string(),
                MessageRole::Function => "user".to_string(),
            }),
        }
    }
}

impl From<GoogleUsageMetadata> for TokenUsage {
    fn from(usage: GoogleUsageMetadata) -> Self {
        Self {
            prompt_tokens: usage.prompt_token_count,
            completion_tokens: usage.candidates_token_count,
            total_tokens: usage.total_token_count,
            estimated_cost: 0.0, // Will be calculated by cost calculator
        }
    }
}

impl GoogleResponse {
    /// Convert to our internal ChatMessage format
    pub fn to_chat_message(&self) -> ChatMessage {
        let content = self.candidates
            .first()
            .and_then(|candidate| {
                candidate.content.as_ref().and_then(|content| {
                    if content.parts.is_empty() {
                        None
                    } else {
                        content.parts.first().map(|part| part.text.clone())
                    }
                })
            })
            .unwrap_or_else(|| "No response generated".to_string());

        ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call: None,
        }
    }

    /// Get finish reason from first candidate
    pub fn get_finish_reason(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|candidate| candidate.finish_reason.clone())
    }
}

/// Helper function to create system prompt content for Google
pub fn create_system_content(system_message: &str) -> GoogleContent {
    GoogleContent {
        parts: vec![GooglePart {
            text: format!("System: {}", system_message),
        }],
        role: Some("user".to_string()),
    }
}

/// Helper function to convert conversation history for Google
pub fn convert_conversation_history(messages: &[ChatMessage]) -> Vec<GoogleContent> {
    let mut contents = Vec::new();
    let mut system_messages = Vec::new();
    
    // Collect system messages separately
    for msg in messages {
        match msg.role {
            MessageRole::System => {
                system_messages.push(msg.content.clone());
            }
            _ => {
                contents.push(GoogleContent::from(msg));
            }
        }
    }
    
    // If we have system messages, prepend them as user content
    if !system_messages.is_empty() {
        let system_content = GoogleContent {
            parts: vec![GooglePart {
                text: format!("System instructions: {}", system_messages.join("\n")),
            }],
            role: Some("user".to_string()),
        };
        contents.insert(0, system_content);
    }
    
    contents
}