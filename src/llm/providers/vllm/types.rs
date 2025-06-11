//! vLLM provider types
//! Since vLLM provides an OpenAI-compatible API, we reuse OpenAI types where possible

use serde::{Deserialize, Serialize};

// Re-export OpenAI types that vLLM is compatible with
pub use crate::llm::providers::openai::types::{
    OpenAIRequest as VLLMRequest,
    OpenAIResponse as VLLMResponse,
    OpenAIChatMessage as VLLMChatMessage,
    OpenAIChoice as VLLMChoice,
    OpenAIUsage as VLLMUsage,
    OpenAIStreamingChunk as VLLMStreamingChunk,
    OpenAIStreamingChoice as VLLMStreamingChoice,
    OpenAIError as VLLMError,
    OpenAIErrorDetails as VLLMErrorDetails,
    OpenAIModel as VLLMModel,
    OpenAIModelsResponse as VLLMModelsResponse,
};

/// vLLM server health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMHealthResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_requests_running: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_requests_waiting: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_cache_usage: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cache_usage: Option<f32>,
}

/// vLLM server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMServerInfo {
    pub version: String,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_parallel_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_parallel_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_memory_utilization: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_seqs: Option<u32>,
}

/// vLLM embeddings request (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMEmbeddingsRequest {
    pub model: String,
    pub input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// vLLM embeddings response (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMEmbeddingsResponse {
    pub object: String,
    pub data: Vec<VLLMEmbedding>,
    pub model: String,
    pub usage: VLLMEmbeddingsUsage,
}

/// vLLM embedding data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMEmbedding {
    pub object: String,
    pub embedding: Vec<f64>,
    pub index: u32,
}

/// vLLM embeddings usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLLMEmbeddingsUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vllm_health_response() {
        let json = r#"{
            "status": "healthy",
            "timestamp": 1677652288,
            "version": "0.2.7",
            "model_name": "meta-llama/Llama-2-7b-chat-hf",
            "num_requests_running": 2,
            "num_requests_waiting": 0,
            "gpu_cache_usage": 0.75,
            "cpu_cache_usage": 0.45
        }"#;

        let health: VLLMHealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, "healthy");
        assert_eq!(health.gpu_cache_usage, Some(0.75));
    }

    #[test]
    fn test_vllm_embeddings_request() {
        let request = VLLMEmbeddingsRequest {
            model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            input: vec!["Hello world".to_string(), "How are you?".to_string()],
            encoding_format: Some("float".to_string()),
            user: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("sentence-transformers"));
        assert!(json.contains("Hello world"));
    }
}