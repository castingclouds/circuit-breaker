//! Agents module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for creating and managing AI agents.

use crate::{schema::QueryBuilder, Client, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, pin::Pin};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};

/// Client for agent operations
#[derive(Debug, Clone)]
pub struct AgentClient {
    client: Client,
}

impl AgentClient {
    /// Create a new agent client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a new agent
    pub fn create(&self) -> AgentBuilder {
        AgentBuilder::new(self.client.clone())
    }

    /// Get an agent by ID
    pub async fn get(&self, id: String) -> Result<Agent> {
        let query = QueryBuilder::query_with_params(
            "GetAgent",
            "agent(id: $id)",
            &[
                "id",
                "name",
                "description",
                "llmProvider { providerType model baseUrl }",
                "llmConfig { temperature maxTokens topP frequencyPenalty presencePenalty stopSequences }",
                "prompts { system userTemplate contextInstructions }",
                "capabilities",
                "tools",
                "createdAt",
                "updatedAt"
            ],
            &[("id", "ID!")],
        );

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            agent: AgentData,
        }

        let response: Response = self.client.graphql(&query, Variables { id }).await?;

        Ok(Agent {
            client: self.client.clone(),
            data: response.agent,
        })
    }

    /// List agents
    /// List all agents
    pub async fn list(&self) -> Result<Vec<Agent>> {
        let query = QueryBuilder::query(
            "ListAgents",
            "agents",
            &[
                "id",
                "name",
                "description",
                "llmProvider { providerType model baseUrl }",
                "llmConfig { temperature maxTokens topP frequencyPenalty presencePenalty stopSequences }",
                "prompts { system userTemplate contextInstructions }",
                "capabilities",
                "tools",
                "createdAt",
                "updatedAt"
            ],
        );

        #[derive(Deserialize)]
        struct Response {
            agents: Vec<AgentData>,
        }

        let response: Response = self.client.graphql(&query, ()).await?;

        Ok(response
            .agents
            .into_iter()
            .map(|data| Agent {
                client: self.client.clone(),
                data,
            })
            .collect())
    }

    /// Execute an agent (non-streaming) as defined in PRD Section 4.1.1
    pub async fn execute(
        &self,
        agent_id: &str,
        request: AgentExecutionRequest,
    ) -> Result<AgentExecutionResponse> {
        #[derive(Serialize)]
        struct BackendRequest {
            context: serde_json::Value,
            input_mapping: Option<std::collections::HashMap<String, String>>,
            output_mapping: Option<std::collections::HashMap<String, String>>,
        }

        #[derive(Deserialize)]
        struct BackendResponse {
            execution_id: String,
            agent_id: String,
            status: String,
            output: Option<serde_json::Value>,
            error: Option<String>,
            created_at: String,
            context: serde_json::Value,
        }

        let backend_request = BackendRequest {
            context: request.context,
            input_mapping: request.mapping.as_ref().and_then(|m| {
                m.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
            }),
            output_mapping: None, // TODO: Extract from mapping if needed
        };

        let url = format!("/agents/{}/execute", agent_id);
        let rest_endpoint = self.client.get_endpoint_url("rest").trim_end_matches('/');
        let mut req = self
            .client
            .http_client()
            .post(&format!("{}{}", rest_endpoint, url))
            .json(&backend_request);

        // Add tenant_id header if provided
        if let Some(tenant_id) = &request.tenant_id {
            req = req.header("X-Tenant-ID", tenant_id);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::Error::Server {
                status: status_code,
                message: error_text,
            });
        }

        let backend_response: BackendResponse = response.json().await?;

        // Parse the created_at timestamp
        let created_at = chrono::DateTime::parse_from_rfc3339(&backend_response.created_at)
            .map_err(|e| crate::Error::Parse {
                message: format!("Invalid timestamp format: {}", e),
            })?
            .with_timezone(&chrono::Utc);

        let status = match backend_response.status.as_str() {
            "pending" => AgentExecutionStatus::Pending,
            "running" => AgentExecutionStatus::Running,
            "completed" => AgentExecutionStatus::Completed,
            "failed" => AgentExecutionStatus::Failed,
            "timeout" => AgentExecutionStatus::Timeout,
            "cancelled" => AgentExecutionStatus::Cancelled,
            _ => AgentExecutionStatus::Pending,
        };

        Ok(AgentExecutionResponse {
            execution_id: backend_response.execution_id,
            agent_id: backend_response.agent_id,
            status,
            context: backend_response.context,
            output: backend_response.output,
            error: backend_response.error,
            created_at,
            completed_at: None, // Backend doesn't return this in execute response
            duration_ms: None,
        })
    }

    /// Execute an agent with streaming as defined in PRD Section 4.1.2
    pub async fn execute_stream(
        &self,
        agent_id: &str,
        request: AgentExecutionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<AgentStreamEvent>> + Send>>> {
        #[derive(Serialize)]
        struct BackendRequest {
            context: serde_json::Value,
            input_mapping: Option<std::collections::HashMap<String, String>>,
            output_mapping: Option<std::collections::HashMap<String, String>>,
        }

        let backend_request = BackendRequest {
            context: request.context,
            input_mapping: request.mapping.as_ref().and_then(|m| {
                m.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
            }),
            output_mapping: None,
        };

        // Use the streaming execution endpoint
        let url = format!("agents/{}/execute/stream", agent_id);
        let full_url = format!("{}{}", self.client.get_endpoint_url("rest"), url);

        let mut req_builder = self
            .client
            .http_client()
            .post(&full_url)
            .json(&backend_request)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Connection", "keep-alive")
            .header("X-Accel-Buffering", "no")
            .header("Transfer-Encoding", "chunked");

        // Add tenant_id header if provided
        if let Some(tenant_id) = &request.tenant_id {
            req_builder = req_builder.header("X-Tenant-ID", tenant_id);
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::Error::Server {
                status: status_code,
                message: error_text,
            });
        }

        // Create raw streaming pipe using text stream for SSE
        use futures::StreamExt;
        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                chunk_result.map_err(|e| crate::Error::Network {
                    message: e.to_string(),
                })
            })
            .map(|chunk_result| {
                chunk_result.and_then(|chunk| {
                    let chunk_str = String::from_utf8_lossy(&chunk);

                    // Create a simple event with the raw content
                    Ok(AgentStreamEvent {
                        event_type: "raw".to_string(),
                        execution_id: "streaming".to_string(),
                        data: serde_json::json!({
                            "raw_content": chunk_str,
                            "byte_count": chunk.len()
                        }),
                        timestamp: chrono::Utc::now(),
                    })
                })
            });

        Ok(Box::pin(stream))
    }

    /// Execute an agent via WebSocket
    pub async fn execute_websocket(
        &self,
        agent_id: &str,
        request: AgentExecutionRequest,
    ) -> Result<AgentWebSocketStream> {
        // Convert HTTP URL to WebSocket URL
        let ws_url = self
            .client
            .get_endpoint_url("rest")
            .to_string()
            .replace("http://", "ws://")
            .replace("https://", "wss://");
        let ws_url = if let Some(tenant_id) = &request.tenant_id {
            format!("{}agents/ws?tenant_id={}", ws_url, tenant_id)
        } else {
            format!("{}agents/ws", ws_url)
        };
        println!("🔍 SDK DEBUG: WebSocket URL: {}", ws_url);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| crate::Error::Network {
                message: format!("Failed to connect to WebSocket: {}", e),
            })?;

        let mut stream = AgentWebSocketStream::new(ws_stream);

        // Authenticate if tenant_id is provided
        if let Some(tenant_id) = &request.tenant_id {
            stream.authenticate(tenant_id.clone()).await?;
        }

        // Execute the agent
        stream.execute_agent(agent_id, request).await?;

        Ok(stream)
    }

    /// List agent executions
    pub async fn list_executions(
        &self,
        agent_id: &str,
        request: ListExecutionsRequest,
    ) -> Result<ListExecutionsResponse> {
        #[derive(Deserialize)]
        struct BackendExecutionSummary {
            execution_id: String,
            agent_id: String,
            status: String,
            created_at: String,
            completed_at: Option<String>,
            has_error: bool,
        }

        #[derive(Deserialize)]
        struct BackendListResponse {
            executions: Vec<BackendExecutionSummary>,
            total: usize,
            page: usize,
            page_size: usize,
        }

        let mut url = format!("/agents/{}/executions", agent_id);
        let mut query_params = Vec::new();

        if let Some(limit) = request.limit {
            query_params.push(format!("limit={}", limit));
        }
        if let Some(offset) = request.offset {
            query_params.push(format!("offset={}", offset));
        }
        if let Some(ref status) = request.status {
            query_params.push(format!("status={}", status));
        }

        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let mut req = self
            .client
            .http_client()
            .get(&format!("{}{}", self.client.base_url(), url));

        // Add tenant_id header if provided
        if let Some(tenant_id) = &request.tenant_id {
            req = req.header("X-Tenant-ID", tenant_id);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::Error::Server {
                status: status_code,
                message: error_text,
            });
        }

        let backend_response: BackendListResponse = response.json().await?;

        let executions = backend_response
            .executions
            .into_iter()
            .map(|exec| {
                let created_at = chrono::DateTime::parse_from_rfc3339(&exec.created_at)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc);

                let completed_at = exec
                    .completed_at
                    .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                let status = match exec.status.as_str() {
                    "pending" => AgentExecutionStatus::Pending,
                    "running" => AgentExecutionStatus::Running,
                    "completed" => AgentExecutionStatus::Completed,
                    "failed" => AgentExecutionStatus::Failed,
                    "timeout" => AgentExecutionStatus::Timeout,
                    "cancelled" => AgentExecutionStatus::Cancelled,
                    _ => AgentExecutionStatus::Pending,
                };

                AgentExecutionResponse {
                    execution_id: exec.execution_id,
                    agent_id: exec.agent_id,
                    status,
                    context: serde_json::json!({}), // Not included in summary
                    output: None,                   // Not included in summary
                    error: if exec.has_error {
                        Some("Error occurred".to_string())
                    } else {
                        None
                    },
                    created_at,
                    completed_at,
                    duration_ms: completed_at
                        .map(|ca| ca.signed_duration_since(created_at).num_milliseconds() as u64),
                }
            })
            .collect();

        Ok(ListExecutionsResponse {
            executions,
            total: backend_response.total as u64,
            limit: backend_response.page_size as u32,
            offset: (backend_response.page * backend_response.page_size) as u32,
        })
    }

    /// Get specific execution details
    pub async fn get_execution(
        &self,
        agent_id: &str,
        execution_id: &str,
        tenant_id: Option<&str>,
    ) -> Result<AgentExecutionResponse> {
        #[derive(Deserialize)]
        struct BackendResponse {
            execution_id: String,
            agent_id: String,
            status: String,
            output: Option<serde_json::Value>,
            error: Option<String>,
            created_at: String,
            context: serde_json::Value,
        }

        let url = format!("/agents/{}/executions/{}", agent_id, execution_id);
        let mut req = self
            .client
            .http_client()
            .get(&format!("{}{}", self.client.base_url(), url));

        // Add tenant_id header if provided
        if let Some(tenant_id) = tenant_id {
            req = req.header("X-Tenant-ID", tenant_id);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::Error::Server {
                status: status_code,
                message: error_text,
            });
        }

        let backend_response: BackendResponse = response.json().await?;

        // Parse the created_at timestamp
        let created_at = chrono::DateTime::parse_from_rfc3339(&backend_response.created_at)
            .map_err(|e| crate::Error::Parse {
                message: format!("Invalid timestamp format: {}", e),
            })?
            .with_timezone(&chrono::Utc);

        let status = match backend_response.status.as_str() {
            "pending" => AgentExecutionStatus::Pending,
            "running" => AgentExecutionStatus::Running,
            "completed" => AgentExecutionStatus::Completed,
            "failed" => AgentExecutionStatus::Failed,
            "timeout" => AgentExecutionStatus::Timeout,
            "cancelled" => AgentExecutionStatus::Cancelled,
            _ => AgentExecutionStatus::Pending,
        };

        Ok(AgentExecutionResponse {
            execution_id: backend_response.execution_id,
            agent_id: backend_response.agent_id,
            status,
            context: backend_response.context,
            output: backend_response.output,
            error: backend_response.error,
            created_at,
            completed_at: None, // Backend doesn't include this in single execution response
            duration_ms: None,
        })
    }
}

/// Builder for creating agents
pub struct AgentBuilder {
    client: Client,
    name: Option<String>,
    description: Option<String>,
    agent_type: Option<String>,
    llm_provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    system_prompt: Option<String>,
    user_template: Option<String>,
    context_instructions: Option<String>,
    capabilities: Vec<String>,
    tools: Vec<ToolDefinition>,
    memory: Option<MemoryConfig>,
    config: serde_json::Value,
}

impl AgentBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            name: None,
            description: None,
            agent_type: None,
            llm_provider: None,
            model: None,
            api_key: None,
            base_url: None,
            temperature: None,
            max_tokens: None,
            system_prompt: None,
            user_template: None,
            context_instructions: None,
            capabilities: Vec::new(),
            tools: Vec::new(),
            memory: None,
            config: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Set the agent name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the agent description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the agent type
    pub fn set_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }

    /// Set conversational agent type
    pub fn conversational(mut self) -> Self {
        self.agent_type = Some("conversational".to_string());
        self
    }

    /// Set tool agent type
    pub fn tool(mut self) -> Self {
        self.agent_type = Some("tool".to_string());
        self
    }

    /// Set the LLM provider
    pub fn set_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set the model
    pub fn set_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature
    pub fn set_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn set_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set system prompt
    pub fn set_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a tool
    pub fn add_tool(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        self.tools.push(ToolDefinition {
            name: name.into(),
            description: description.into(),
            parameters,
        });
        self
    }

    /// Set memory configuration
    pub fn set_memory(mut self, memory_type: impl Into<String>, config: serde_json::Value) -> Self {
        self.memory = Some(MemoryConfig {
            memory_type: memory_type.into(),
            config,
        });
        self
    }

    /// Set the API key for LLM provider
    pub fn set_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the base URL for LLM provider
    pub fn set_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Set the user template for prompts
    pub fn set_user_template(mut self, template: impl Into<String>) -> Self {
        self.user_template = Some(template.into());
        self
    }

    /// Set context instructions for prompts
    pub fn set_context_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.context_instructions = Some(instructions.into());
        self
    }

    /// Add a capability
    pub fn add_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Set the agent configuration
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    /// Build and create the agent
    pub async fn build(self) -> Result<Agent> {
        let name = self.name.ok_or_else(|| crate::Error::Validation {
            message: "Agent name is required".to_string(),
        })?;

        let _agent_type = self.agent_type.ok_or_else(|| crate::Error::Validation {
            message: "Agent type is required".to_string(),
        })?;

        // Build comprehensive config from individual settings
        let mut config = if self.config.is_object() {
            self.config.as_object().unwrap().clone()
        } else {
            serde_json::Map::new()
        };

        if let Some(ref provider) = self.llm_provider {
            config.insert(
                "llm_provider".to_string(),
                serde_json::Value::String(provider.clone()),
            );
        }
        if let Some(ref model) = self.model {
            config.insert(
                "model".to_string(),
                serde_json::Value::String(model.clone()),
            );
        }
        if let Some(temperature) = self.temperature {
            config.insert(
                "temperature".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(temperature as f64).unwrap(),
                ),
            );
        }
        if let Some(max_tokens) = self.max_tokens {
            config.insert(
                "max_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(max_tokens)),
            );
        }
        if let Some(ref system_prompt) = self.system_prompt {
            config.insert(
                "system_prompt".to_string(),
                serde_json::Value::String(system_prompt.clone()),
            );
        }
        if !self.tools.is_empty() {
            config.insert(
                "tools".to_string(),
                serde_json::to_value(&self.tools).unwrap(),
            );
        }
        if let Some(memory) = self.memory {
            config.insert("memory".to_string(), serde_json::to_value(&memory).unwrap());
        }

        let mutation = QueryBuilder::mutation_with_params(
            "CreateAgent",
            "createAgent(input: $input)",
            &[
                "id",
                "name",
                "description",
                "llmProvider { providerType model baseUrl }",
                "llmConfig { temperature maxTokens topP frequencyPenalty presencePenalty stopSequences }",
                "prompts { system userTemplate contextInstructions }",
                "capabilities",
                "tools",
                "createdAt",
                "updatedAt"
            ],
            &[("input", "AgentDefinitionInput!")],
        );

        #[derive(Serialize)]
        struct Variables {
            input: AgentDefinitionInput,
        }

        #[derive(Serialize)]
        struct AgentDefinitionInput {
            name: String,
            description: String,
            #[serde(rename = "llmProvider")]
            llm_provider: AgentLLMProviderInput,
            #[serde(rename = "llmConfig")]
            llm_config: LLMConfigInput,
            prompts: AgentPromptsInput,
            capabilities: Vec<String>,
            tools: Vec<String>,
        }

        #[derive(Serialize)]
        struct AgentLLMProviderInput {
            #[serde(rename = "providerType")]
            provider_type: String,
            model: String,
            #[serde(rename = "apiKey")]
            api_key: String,
            #[serde(rename = "baseUrl")]
            base_url: Option<String>,
        }

        #[derive(Serialize)]
        struct LLMConfigInput {
            temperature: f64,
            #[serde(rename = "maxTokens")]
            max_tokens: Option<i32>,
            #[serde(rename = "topP")]
            top_p: Option<f64>,
            #[serde(rename = "frequencyPenalty")]
            frequency_penalty: Option<f64>,
            #[serde(rename = "presencePenalty")]
            presence_penalty: Option<f64>,
            #[serde(rename = "stopSequences")]
            stop_sequences: Vec<String>,
        }

        #[derive(Serialize)]
        struct AgentPromptsInput {
            system: String,
            #[serde(rename = "userTemplate")]
            user_template: String,
            #[serde(rename = "contextInstructions")]
            context_instructions: Option<String>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createAgent")]
            create_agent: AgentData,
        }

        let response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    input: AgentDefinitionInput {
                        name,
                        description: self.description.unwrap_or_else(|| "".to_string()),
                        llm_provider: AgentLLMProviderInput {
                            provider_type: self
                                .llm_provider
                                .clone()
                                .unwrap_or_else(|| "openai".to_string()),
                            model: self
                                .model
                                .clone()
                                .unwrap_or_else(|| "gpt-4o-mini".to_string()),
                            api_key: self.api_key.clone().unwrap_or_else(|| "".to_string()),
                            base_url: self.base_url.clone(),
                        },
                        llm_config: LLMConfigInput {
                            temperature: self.temperature.unwrap_or(0.7) as f64,
                            max_tokens: self.max_tokens.map(|t| t as i32),
                            top_p: None,
                            frequency_penalty: None,
                            presence_penalty: None,
                            stop_sequences: Vec::new(),
                        },
                        prompts: AgentPromptsInput {
                            system: self
                                .system_prompt
                                .clone()
                                .unwrap_or_else(|| "You are a helpful assistant.".to_string()),
                            user_template: self
                                .user_template
                                .clone()
                                .unwrap_or_else(|| "{message}".to_string()),
                            context_instructions: self.context_instructions.clone(),
                        },
                        capabilities: self.capabilities,
                        tools: self.tools.into_iter().map(|t| t.name).collect(),
                    },
                },
            )
            .await?;

        Ok(Agent {
            client: self.client,
            data: response.create_agent,
        })
    }
}

/// An agent instance
#[derive(Debug, Clone)]
pub struct Agent {
    client: Client,
    data: AgentData,
}

impl Agent {
    /// Get the agent ID
    pub fn id(&self) -> String {
        self.data.id.clone()
    }

    /// Get the agent name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// Get the agent description
    pub fn description(&self) -> &str {
        &self.data.description
    }

    /// Get the agent's LLM provider type
    pub fn provider_type(&self) -> &str {
        &self.data.llm_provider.provider_type
    }

    /// Get the agent's model
    pub fn model(&self) -> &str {
        &self.data.llm_provider.model
    }

    /// Send a message to the agent (for conversational agents)
    pub async fn send_message(&self, message: impl Into<String>) -> Result<String> {
        let message = message.into();

        // Build the messages array with system prompt and user message
        let mut messages = vec![crate::llm::ChatMessage {
            role: crate::llm::ChatRole::System,
            content: self.data.prompts.system.clone(),
            name: None,
        }];

        // Add user message
        messages.push(crate::llm::ChatMessage {
            role: crate::llm::ChatRole::User,
            content: message,
            name: None,
        });

        // Create LLM client and make REST API call
        let llm_client = self.client.llm();
        let request = crate::llm::ChatCompletionRequest {
            model: self.data.llm_provider.model.clone(),
            messages,
            temperature: Some(self.data.llm_config.temperature as f32),
            max_tokens: self.data.llm_config.max_tokens.map(|t| t as u32),
            stream: Some(true),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
            functions: None,
            function_call: None,
            circuit_breaker: None,
        };

        let response = llm_client.chat_completion(request).await?;

        // Extract the assistant's response
        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(crate::Error::Network {
                message: "No response from agent".to_string(),
            })
        }
    }

    /// Delete the agent
    pub async fn delete(self) -> Result<()> {
        let mutation = QueryBuilder::mutation_with_params(
            "DeleteAgent",
            "deleteAgent(id: $id)",
            &["success"],
            &[("id", "ID!")],
        );

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "deleteAgent")]
            delete_agent: DeleteResult,
        }

        #[derive(Deserialize)]
        struct DeleteResult {
            success: bool,
        }

        let _response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    id: self.data.id.clone(),
                },
            )
            .await?;

        Ok(())
    }
}

// Internal data structures
#[derive(Debug, Clone, Deserialize)]
struct AgentData {
    id: String,
    name: String,
    description: String,
    #[serde(rename = "llmProvider")]
    llm_provider: AgentLLMProviderData,
    #[serde(rename = "llmConfig")]
    llm_config: LLMConfigData,
    prompts: AgentPromptsData,
    capabilities: Vec<String>,
    tools: Vec<String>,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentLLMProviderData {
    #[serde(rename = "providerType")]
    provider_type: String,
    model: String,
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LLMConfigData {
    temperature: f64,
    #[serde(rename = "maxTokens")]
    max_tokens: Option<i32>,
    #[serde(rename = "topP")]
    top_p: Option<f64>,
    #[serde(rename = "frequencyPenalty")]
    frequency_penalty: Option<f64>,
    #[serde(rename = "presencePenalty")]
    presence_penalty: Option<f64>,
    #[serde(rename = "stopSequences")]
    stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentPromptsData {
    system: String,
    #[serde(rename = "userTemplate")]
    user_template: String,
    #[serde(rename = "contextInstructions")]
    context_instructions: Option<String>,
}

/// Tool definition for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Memory configuration for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub memory_type: String,
    pub config: serde_json::Value,
}

/// Convenience function to create an agent builder
pub fn create_agent(name: impl Into<String>) -> AgentBuilderStandalone {
    AgentBuilderStandalone::new(name.into())
}

/// Standalone agent builder that can be used without a client initially
pub struct AgentBuilderStandalone {
    name: String,
    description: Option<String>,
    agent_type: Option<String>,
    llm_provider: Option<String>,
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    system_prompt: Option<String>,
    tools: Vec<ToolDefinition>,
    memory: Option<MemoryConfig>,
}

impl AgentBuilderStandalone {
    fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            agent_type: None,
            llm_provider: None,
            model: None,
            temperature: None,
            max_tokens: None,
            system_prompt: None,
            tools: Vec::new(),
            memory: None,
        }
    }

    /// Set the agent description
    pub fn set_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the agent type
    pub fn set_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }

    /// Set the LLM provider
    pub fn set_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set the model
    pub fn set_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the temperature
    pub fn set_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn set_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set system prompt
    pub fn set_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a tool
    pub fn add_tool(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        self.tools.push(ToolDefinition {
            name: name.into(),
            description: description.into(),
            parameters,
        });
        self
    }

    /// Set memory configuration
    pub fn set_memory(mut self, memory_type: impl Into<String>, config: serde_json::Value) -> Self {
        self.memory = Some(MemoryConfig {
            memory_type: memory_type.into(),
            config,
        });
        self
    }

    /// Build the agent definition
    pub fn build(self) -> AgentDefinition {
        AgentDefinition {
            name: self.name,
            description: self.description,
            agent_type: self.agent_type,
            llm_provider: self.llm_provider,
            model: self.model,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            system_prompt: self.system_prompt,
            tools: self.tools,
            memory: self.memory,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentDefinition {
    pub name: String,
    pub description: Option<String>,
    pub agent_type: Option<String>,
    pub llm_provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub memory: Option<MemoryConfig>,
}

/// Agent execution request as defined in the PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionRequest {
    pub context: serde_json::Value,
    pub mapping: Option<serde_json::Value>,
    pub tenant_id: Option<String>,
    pub stream: Option<bool>,
}

/// Agent execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResponse {
    pub execution_id: String,
    pub agent_id: String,
    pub status: AgentExecutionStatus,
    pub context: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

/// Agent execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

/// Agent stream event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamEvent {
    pub event_type: String,
    pub execution_id: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Request for listing agent executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListExecutionsRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
    pub tenant_id: Option<String>,
}

/// Response for listing agent executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListExecutionsResponse {
    pub executions: Vec<AgentExecutionResponse>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

/// WebSocket message types for agent communication (matches backend ServerMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentWebSocketServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess { tenant_id: String },
    #[serde(rename = "auth_failure")]
    AuthFailure { error: String },
    #[serde(rename = "execution_started")]
    ExecutionStarted {
        execution_id: String,
        agent_id: String,
        timestamp: String,
    },
    #[serde(rename = "thinking")]
    Thinking {
        execution_id: String,
        status: String,
        timestamp: String,
    },
    #[serde(rename = "chunk")]
    ContentChunk {
        execution_id: String,
        chunk: String,
        sequence: u32,
        timestamp: String,
    },
    #[serde(rename = "complete")]
    Complete {
        execution_id: String,
        response: serde_json::Value,
        usage: Option<serde_json::Value>,
        timestamp: String,
    },
    #[serde(rename = "error")]
    Error {
        execution_id: Option<String>,
        error: String,
        timestamp: String,
    },
    #[serde(rename = "pong")]
    Pong { timestamp: String },
}

/// WebSocket client message types (matches backend ClientMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentWebSocketClientMessage {
    #[serde(rename = "auth")]
    Authenticate { token: String },
    #[serde(rename = "subscribe")]
    Subscribe { execution_id: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { execution_id: String },
    #[serde(rename = "execute")]
    ExecuteAgent {
        agent_id: String,
        context: serde_json::Value,
        input_mapping: Option<HashMap<String, String>>,
        output_mapping: Option<HashMap<String, String>>,
    },
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket stream for agent execution
pub struct AgentWebSocketStream {
    ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    authenticated: bool,
}

impl AgentWebSocketStream {
    pub fn new(
        ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> Self {
        Self {
            ws_stream,
            authenticated: false,
        }
    }

    pub async fn authenticate(&mut self, tenant_id: String) -> Result<()> {
        let auth_msg = AgentWebSocketClientMessage::Authenticate {
            token: tenant_id, // Using tenant_id as token for now
        };

        self.send_message(auth_msg).await?;

        // Wait for auth response
        if let Some(msg) = self.receive_message().await? {
            match msg {
                AgentWebSocketServerMessage::AuthSuccess { .. } => {
                    self.authenticated = true;
                    Ok(())
                }
                AgentWebSocketServerMessage::AuthFailure { error } => {
                    Err(crate::Error::Auth { message: error })
                }
                _ => Err(crate::Error::Network {
                    message: "Unexpected message during authentication".to_string(),
                }),
            }
        } else {
            Err(crate::Error::Network {
                message: "No response to authentication".to_string(),
            })
        }
    }

    pub async fn execute_agent(
        &mut self,
        agent_id: &str,
        request: AgentExecutionRequest,
    ) -> Result<()> {
        let execute_msg = AgentWebSocketClientMessage::ExecuteAgent {
            agent_id: agent_id.to_string(),
            context: request.context,
            input_mapping: request.mapping.as_ref().and_then(|m| {
                m.as_object().map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
            }),
            output_mapping: None, // TODO: Extract from mapping if needed
        };

        self.send_message(execute_msg).await
    }

    pub async fn subscribe(&mut self, execution_id: String) -> Result<()> {
        let subscribe_msg = AgentWebSocketClientMessage::Subscribe { execution_id };
        self.send_message(subscribe_msg).await
    }

    pub async fn unsubscribe(&mut self, execution_id: String) -> Result<()> {
        let unsubscribe_msg = AgentWebSocketClientMessage::Unsubscribe { execution_id };
        self.send_message(unsubscribe_msg).await
    }

    pub async fn ping(&mut self) -> Result<()> {
        let ping_msg = AgentWebSocketClientMessage::Ping;
        self.send_message(ping_msg).await
    }

    async fn send_message(&mut self, message: AgentWebSocketClientMessage) -> Result<()> {
        let json_msg = serde_json::to_string(&message).map_err(|e| crate::Error::Parse {
            message: format!("Failed to serialize message: {}", e),
        })?;

        self.ws_stream
            .send(Message::Text(json_msg))
            .await
            .map_err(|e| crate::Error::Network {
                message: format!("Failed to send WebSocket message: {}", e),
            })
    }

    pub async fn receive_message(&mut self) -> Result<Option<AgentWebSocketServerMessage>> {
        loop {
            match self.ws_stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let message = serde_json::from_str(&text).map_err(|e| crate::Error::Parse {
                        message: format!("Failed to parse WebSocket message: {}", e),
                    })?;
                    return Ok(Some(message));
                }
                Some(Ok(Message::Close(_))) => return Ok(None),
                Some(Ok(_)) => {
                    // Ignore binary, ping, pong messages and continue loop
                    continue;
                }
                Some(Err(e)) => {
                    return Err(crate::Error::Network {
                        message: format!("WebSocket error: {}", e),
                    });
                }
                None => return Ok(None),
            }
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.ws_stream
            .close(None)
            .await
            .map_err(|e| crate::Error::Network {
                message: format!("Failed to close WebSocket: {}", e),
            })
    }
}
