// OpenAI-compatible REST API handlers
// This module implements the actual HTTP handlers for OpenAI-compatible endpoints

use axum::{
    extract::State,
    http::{StatusCode, HeaderMap, header},
    response::{Response, IntoResponse},
    Json,
};
use futures::StreamExt;
use axum::body::Body;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, error, debug};

use super::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatCompletionStreamResponse,
    ChatCompletionChoice, ChatCompletionStreamChoice, ChatMessage, ChatMessageDelta,
    ChatRole, ModelsResponse, Model, ErrorResponse, Usage,
    create_error_response, generate_completion_id, current_timestamp,
    CircuitBreakerConfig, is_virtual_model, get_virtual_models,
};
use crate::llm::{
    LLMRequest, LLMProviderType, LLMRouter,
    cost::CostOptimizer, MessageRole,
};

/// Shared application state for the OpenAI API
#[derive(Clone)]
pub struct OpenAIApiState {
    pub llm_router: Arc<LLMRouter>,
    pub cost_optimizer: Arc<RwLock<CostOptimizer>>,
    pub api_keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
    pub models: Arc<RwLock<Vec<ModelConfig>>>,
}

/// API key information
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub provider_keys: HashMap<LLMProviderType, String>,
    pub usage_limits: Option<UsageLimits>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

/// Usage limits for API keys
#[derive(Debug, Clone)]
pub struct UsageLimits {
    pub daily_tokens: Option<u64>,
    pub monthly_cost: Option<f64>,
    pub rate_limit_per_minute: Option<u32>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub provider: LLMProviderType,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub cost_per_input_token: f64,
    pub cost_per_output_token: f64,
}

impl OpenAIApiState {
    pub fn new() -> Self {
        // Create LLM router (async constructor)
        let llm_router = Arc::new(
            futures::executor::block_on(LLMRouter::new())
                .unwrap_or_else(|_| panic!("Failed to create LLM router"))
        );
        
        // Create cost optimizer with default dependencies
        let usage_tracker = Arc::new(crate::llm::cost::InMemoryUsageTracker::new());
        let budget_manager = Arc::new(crate::llm::cost::BudgetManager::new(usage_tracker));
        let cost_analyzer = Arc::new(crate::llm::cost::CostAnalyzer::new());
        let cost_optimizer = Arc::new(RwLock::new(CostOptimizer::new(budget_manager, cost_analyzer)));
        let api_keys = Arc::new(RwLock::new(HashMap::new()));
        let models = Arc::new(RwLock::new(Self::default_models()));

        Self {
            llm_router,
            cost_optimizer,
            api_keys,
            models,
        }
    }

    fn default_models() -> Vec<ModelConfig> {
        let mut models = vec![
            // Real provider models
            ModelConfig {
                id: "claude-3-haiku-20240307".to_string(),
                provider: LLMProviderType::Anthropic,
                display_name: "Claude 3 Haiku".to_string(),
                context_window: 200000,
                max_output_tokens: 4096,
                supports_streaming: true,
                cost_per_input_token: 0.00000025,
                cost_per_output_token: 0.00000125,
            },
            ModelConfig {
                id: "claude-3-sonnet-20240229".to_string(),
                provider: LLMProviderType::Anthropic,
                display_name: "Claude 3 Sonnet".to_string(),
                context_window: 200000,
                max_output_tokens: 4096,
                supports_streaming: true,
                cost_per_input_token: 0.000003,
                cost_per_output_token: 0.000015,
            },
            ModelConfig {
                id: "claude-sonnet-4-20250514".to_string(),
                provider: LLMProviderType::Anthropic,
                display_name: "Claude 4 Sonnet".to_string(),
                context_window: 200000,
                max_output_tokens: 8192,
                supports_streaming: true,
                cost_per_input_token: 0.000003,
                cost_per_output_token: 0.000015,
            },
            ModelConfig {
                id: "gpt-4".to_string(),
                provider: LLMProviderType::OpenAI,
                display_name: "GPT-4".to_string(),
                context_window: 8192,
                max_output_tokens: 4096,
                supports_streaming: true,
                cost_per_input_token: 0.00003,
                cost_per_output_token: 0.00006,
            },
            ModelConfig {
                id: "gpt-3.5-turbo".to_string(),
                provider: LLMProviderType::OpenAI,
                display_name: "GPT-3.5 Turbo".to_string(),
                context_window: 16385,
                max_output_tokens: 4096,
                supports_streaming: true,
                cost_per_input_token: 0.000001,
                cost_per_output_token: 0.000002,
            },
        ];

        // Add virtual models for smart routing
        for virtual_model in get_virtual_models() {
            models.push(ModelConfig {
                id: virtual_model.id,
                provider: LLMProviderType::Custom("smart-routing".to_string()),
                display_name: virtual_model.description,
                context_window: 200000, // Max context for virtual models
                max_output_tokens: 4096,
                supports_streaming: true,
                cost_per_input_token: virtual_model.max_cost.unwrap_or(0.000001),
                cost_per_output_token: virtual_model.max_cost.unwrap_or(0.000002),
            });
        }

        models
    }

    /// Extract API key from headers
    async fn extract_api_key(&self, headers: &HeaderMap) -> Result<Option<ApiKeyInfo>, ErrorResponse> {
        let auth_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        if let Some(token) = auth_header {
            let api_keys = self.api_keys.read().await;
            Ok(api_keys.get(token).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get model configuration by ID
    async fn get_model(&self, model_id: &str) -> Option<ModelConfig> {
        let models = self.models.read().await;
        models.iter().find(|m| m.id == model_id).cloned()
    }
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "circuit-breaker-openai-api",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": current_timestamp()
    }))
}

/// List available models endpoint - GET /v1/models
pub async fn list_models(
    State(state): State<OpenAIApiState>,
) -> Result<Json<ModelsResponse>, ErrorResponse> {
    debug!("Listing available models");

    let models = state.models.read().await;
    let model_list: Vec<Model> = models
        .iter()
        .map(|config| Model {
            id: config.id.clone(),
            object: "model".to_string(),
            created: current_timestamp(),
            owned_by: "circuit-breaker".to_string(),
            extra: HashMap::from([
                ("provider".to_string(), serde_json::Value::String(config.provider.to_string())),
                ("context_window".to_string(), serde_json::Value::Number(config.context_window.into())),
                ("max_output_tokens".to_string(), serde_json::Value::Number(config.max_output_tokens.into())),
                ("supports_streaming".to_string(), serde_json::Value::Bool(config.supports_streaming)),
            ]),
        })
        .collect();

    Ok(Json(ModelsResponse {
        object: "list".to_string(),
        data: model_list,
    }))
}

/// Chat completions endpoint - POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<OpenAIApiState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, ErrorResponse> {
    debug!("Processing chat completion request for model: {}", request.model);

    // Extract API key (optional for some deployments)
    let _api_key_info = state.extract_api_key(&headers).await?;

    // Extract Circuit Breaker config from request
    let cb_config = request.circuit_breaker.clone();
    
    // Check if smart routing should be used
    let use_smart_routing = cb_config.is_some() || is_virtual_model(&request.model);
    
    // For non-virtual models, validate they exist
    if !is_virtual_model(&request.model) {
        let _model_config = state
            .get_model(&request.model)
            .await
            .ok_or_else(|| {
                create_error_response(
                    format!("Model '{}' not found", request.model),
                    "invalid_request_error".to_string(),
                    Some("model".to_string()),
                    None,
                )
            })?;
    }

    // Convert to internal request format
    let llm_request: LLMRequest = request.clone().into();

    // Check if streaming is requested
    if request.stream {
        if use_smart_routing {
            handle_smart_streaming_completion(state, request, cb_config, llm_request).await
        } else {
            let model_config = state.get_model(&request.model).await.unwrap();
            handle_streaming_completion(state, request, model_config, llm_request).await
        }
    } else {
        if use_smart_routing {
            handle_smart_regular_completion(state, request, cb_config, llm_request).await
        } else {
            let model_config = state.get_model(&request.model).await.unwrap();
            handle_regular_completion(state, request, model_config, llm_request).await
        }
    }
}

/// Handle regular (non-streaming) chat completion
async fn handle_regular_completion(
    state: OpenAIApiState,
    request: ChatCompletionRequest,
    model_config: ModelConfig,
    llm_request: LLMRequest,
) -> Result<Response, ErrorResponse> {
    info!("Processing regular completion for model: {}", request.model);

    // Route the request through the LLM router
    let response = state
        .llm_router
        .chat_completion(llm_request)
        .await
        .map_err(|e| {
            error!("LLM routing failed: {}", e);
            create_error_response(
                format!("Failed to process request: {}", e),
                "internal_error".to_string(),
                None,
                None,
            )
        })?;

    // Convert to OpenAI format
    let completion_id = generate_completion_id();
    let created = current_timestamp();

    let openai_response = ChatCompletionResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: request.model.clone(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: response.choices.first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: response.choices.first()
                .and_then(|c| c.finish_reason.clone()),
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
        },
        system_fingerprint: Some("circuit-breaker-v1".to_string()),
    };

    // Track costs
    {
        let _cost_optimizer = state.cost_optimizer.write().await;
        let estimated_cost = (response.usage.prompt_tokens as f64 * model_config.cost_per_input_token)
            + (response.usage.completion_tokens as f64 * model_config.cost_per_output_token);

        debug!("Estimated cost: ${:.4}", estimated_cost);
    }

    Ok(Json(openai_response).into_response())
}

/// Handle streaming chat completion
async fn handle_streaming_completion(
    state: OpenAIApiState,
    request: ChatCompletionRequest,
    _model_config: ModelConfig,
    llm_request: LLMRequest,
) -> Result<Response, ErrorResponse> {
    use futures::StreamExt;
    
    debug!("Starting streaming completion for model: {}", request.model);

    // Get the LLM router stream
    let router = &state.llm_router;
    let stream_result = router.chat_completion_stream(llm_request).await;

    let mut stream = match stream_result {
        Ok(stream) => stream,
        Err(e) => {
            return Err(create_error_response(
                format!("Failed to start stream: {}", e),
                "internal_error".to_string(),
                Some("stream".to_string()),
                None,
            ));
        }
    };

    // Create manual SSE response
    let (mut sender, body) = Body::channel();
    
    tokio::spawn(async move {
        // Send SSE headers manually
        let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
        let _ = sender.send_data(headers.into()).await;
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(streaming_chunk) => {
                    let sse_data = ChatCompletionStreamResponse {
                        id: streaming_chunk.id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: streaming_chunk.created,
                        model: streaming_chunk.model.clone(),
                        system_fingerprint: None,
                        choices: streaming_chunk.choices.into_iter().map(|choice| {
                            ChatCompletionStreamChoice {
                                index: choice.index,
                                delta: ChatMessageDelta {
                                    role: Some(match choice.delta.role {
                                        MessageRole::User => ChatRole::User,
                                        MessageRole::Assistant => ChatRole::Assistant,
                                        MessageRole::System => ChatRole::System,
                                        MessageRole::Function => ChatRole::Assistant,
                                    }),
                                    content: if choice.delta.content.is_empty() { 
                                        None 
                                    } else { 
                                        Some(choice.delta.content) 
                                    },
                                    tool_calls: None,
                                },
                                logprobs: None,
                                finish_reason: choice.finish_reason,
                            }
                        }).collect(),
                    };
                    
                    if let Ok(json_str) = serde_json::to_string(&sse_data) {
                        let sse_line = format!("data: {}\n\n", json_str);
                        if sender.send_data(sse_line.into()).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_data = format!("data: {{\"error\": \"{}\", \"type\": \"stream_error\"}}\n\n", e);
                    let _ = sender.send_data(error_data.into()).await;
                    break;
                }
            }
        }
        
        // Send final done message
        let _ = sender.send_data("data: [DONE]\n\n".into()).await;
    });

    let response = Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap();
        
    Ok(response.into_response())
}

/// Handle smart streaming completion
async fn handle_smart_streaming_completion(
    state: OpenAIApiState,
    request: ChatCompletionRequest,
    cb_config: Option<CircuitBreakerConfig>,
    llm_request: LLMRequest,
) -> Result<Response, ErrorResponse> {
    use futures::StreamExt;
    
    debug!("Starting smart streaming completion for model: {}", request.model);

    // Get the LLM router stream with smart routing
    let router = &state.llm_router;
    let stream_result = router.smart_chat_completion_stream(llm_request, cb_config).await;

    let mut stream = match stream_result {
        Ok(stream) => stream,
        Err(e) => {
            return Err(create_error_response(
                format!("Failed to start smart stream: {}", e),
                "internal_error".to_string(),
                Some("stream".to_string()),
                None,
            ));
        }
    };

    // Create manual SSE response
    let (mut sender, body) = Body::channel();
    
    tokio::spawn(async move {
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(streaming_chunk) => {
                    let sse_data = ChatCompletionStreamResponse {
                        id: streaming_chunk.id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: streaming_chunk.created,
                        model: streaming_chunk.model.clone(),
                        system_fingerprint: None,
                        choices: streaming_chunk.choices.into_iter().map(|choice| {
                            ChatCompletionStreamChoice {
                                index: choice.index,
                                delta: ChatMessageDelta {
                                    role: Some(match choice.delta.role {
                                        MessageRole::User => ChatRole::User,
                                        MessageRole::Assistant => ChatRole::Assistant,
                                        MessageRole::System => ChatRole::System,
                                        MessageRole::Function => ChatRole::Assistant,
                                    }),
                                    content: if choice.delta.content.is_empty() { 
                                        None 
                                    } else { 
                                        Some(choice.delta.content) 
                                    },
                                    tool_calls: None,
                                },
                                logprobs: None,
                                finish_reason: choice.finish_reason,
                            }
                        }).collect(),
                    };
                    
                    if let Ok(json_str) = serde_json::to_string(&sse_data) {
                        let sse_line = format!("data: {}\n\n", json_str);
                        if sender.send_data(sse_line.into()).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_data = format!("data: {{\"error\": \"{}\", \"type\": \"stream_error\"}}\n\n", e);
                    let _ = sender.send_data(error_data.into()).await;
                    break;
                }
            }
        }
        
        // Send final done message
        let _ = sender.send_data("data: [DONE]\n\n".into()).await;
    });

    let response = Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap();
        
    Ok(response.into_response())
}

/// Get model information endpoint - GET /v1/models/{model_id}
pub async fn get_model(
    State(state): State<OpenAIApiState>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> Result<Json<Model>, ErrorResponse> {
    debug!("Getting model information for: {}", model_id);

    let model_config = state
        .get_model(&model_id)
        .await
        .ok_or_else(|| {
            create_error_response(
                format!("Model '{}' not found", model_id),
                "invalid_request_error".to_string(),
                Some("model".to_string()),
                None,
            )
        })?;

    let model = Model {
        id: model_config.id.clone(),
        object: "model".to_string(),
        created: current_timestamp(),
        owned_by: "circuit-breaker".to_string(),
        extra: HashMap::from([
            ("provider".to_string(), serde_json::Value::String(model_config.provider.to_string())),
            ("context_window".to_string(), serde_json::Value::Number(model_config.context_window.into())),
            ("max_output_tokens".to_string(), serde_json::Value::Number(model_config.max_output_tokens.into())),
            ("supports_streaming".to_string(), serde_json::Value::Bool(model_config.supports_streaming)),
            ("cost_per_input_token".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(model_config.cost_per_input_token).unwrap_or_else(|| serde_json::Number::from(0)))),
            ("cost_per_output_token".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(model_config.cost_per_output_token).unwrap_or_else(|| serde_json::Number::from(0)))),
        ]),
    };

    Ok(Json(model))
}

/// Handle smart routing for regular (non-streaming) completion
async fn handle_smart_regular_completion(
    state: OpenAIApiState,
    request: ChatCompletionRequest,
    cb_config: Option<CircuitBreakerConfig>,
    llm_request: LLMRequest,
) -> Result<Response, ErrorResponse> {
    info!("Processing smart regular completion for model: {}", request.model);

    // Use smart routing
    let response = state
        .llm_router
        .smart_chat_completion(llm_request, cb_config.clone())
        .await
        .map_err(|e| {
            error!("Smart LLM routing failed: {}", e);
            create_error_response(
                format!("Failed to process smart request: {}", e),
                "internal_error".to_string(),
                None,
                None,
            )
        })?;

    // Convert to OpenAI format
    let completion_id = generate_completion_id();
    let created = current_timestamp();

    let openai_response = ChatCompletionResponse {
        id: completion_id,
        object: "chat.completion".to_string(),
        created,
        model: response.routing_info.selected_provider.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: ChatRole::Assistant,
                content: response.choices.first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: response.choices.first()
                .and_then(|c| c.finish_reason.clone()),
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
        },
        system_fingerprint: Some("circuit-breaker-smart-v1".to_string()),
    };

    // Add routing information to response metadata
    if let Some(config) = cb_config {
        // Add smart routing metadata to the response
        debug!("Smart routing used strategy: {:?}", config.routing_strategy);
        debug!("Selected provider: {}", response.routing_info.selected_provider);
    }

    Ok(Json(openai_response).into_response())
}

/// Handle smart routing for streaming completion (temporarily disabled for compilation)


/// Error handler for invalid routes
pub async fn not_found() -> impl IntoResponse {
    let error = create_error_response(
        "Not found".to_string(),
        "invalid_request_error".to_string(),
        None,
        None,
    );
    (StatusCode::NOT_FOUND, Json(error))
}

/// Error response implementation
impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.error.error_type.as_str() {
            "invalid_request_error" => StatusCode::BAD_REQUEST,
            "authentication_error" => StatusCode::UNAUTHORIZED,
            "permission_error" => StatusCode::FORBIDDEN,
            "not_found_error" => StatusCode::NOT_FOUND,
            "rate_limit_error" => StatusCode::TOO_MANY_REQUESTS,
            "internal_error" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };

        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_model_config_creation() {
        let models = OpenAIApiState::default_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "gpt-4"));
        assert!(models.iter().any(|m| m.id == "claude-3-haiku-20240307"));
        assert!(models.iter().any(|m| m.id == "claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_completion_id_format() {
        let id = generate_completion_id();
        assert!(id.starts_with("chatcmpl-"));
        assert_eq!(id.len(), 36);
    }
}