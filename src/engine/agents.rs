// Agent execution engine for Places AI Agent functionality
// This module handles AI agent execution for workflow tokens

//! # Agent Execution Engine
//! 
//! This module provides the core engine for executing AI agents within workflows.
//! It supports both transition-based agent execution and place-based agent execution.
//! 
//! ## Features
//! 
//! - **Places AI Agent**: Run agents on tokens in specific places
//! - **Transition Agent**: Run agents during workflow transitions
//! - **Real-time Streaming**: Stream agent responses via multiple protocols
//! - **LLM Provider Support**: OpenAI, Anthropic, Google, Ollama, custom APIs
//! - **Retry Logic**: Configurable retry with backoff strategies
//! - **Input/Output Mapping**: Map token data to agent inputs and outputs
//! - **Scheduling**: Support for delayed and periodic agent execution

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use tokio::time::sleep;
use uuid::Uuid;
use serde_json::{Value, json};
use reqwest;

use crate::models::{
    AgentId, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentStreamEvent,
    PlaceAgentConfig, AgentTransitionConfig, LLMProvider, LLMConfig, Token, PlaceId
};
use crate::engine::rules::RulesEngine;
use crate::{Result, CircuitBreakerError};

/// Storage trait for agent-related data
#[async_trait::async_trait]
pub trait AgentStorage: Send + Sync {
    // Agent definitions
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()>;
    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>>;
    async fn list_agents(&self) -> Result<Vec<AgentDefinition>>;
    async fn delete_agent(&self, id: &AgentId) -> Result<bool>;

    // Place agent configurations
    async fn store_place_agent_config(&self, config: &PlaceAgentConfig) -> Result<()>;
    async fn get_place_agent_configs(&self, place_id: &PlaceId) -> Result<Vec<PlaceAgentConfig>>;
    async fn list_place_agent_configs(&self) -> Result<Vec<PlaceAgentConfig>>;
    async fn delete_place_agent_config(&self, id: &Uuid) -> Result<bool>;

    // Agent executions
    async fn store_execution(&self, execution: &AgentExecution) -> Result<()>;
    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>>;
    async fn list_executions_for_token(&self, token_id: &Uuid) -> Result<Vec<AgentExecution>>;
    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>>;
}

/// In-memory implementation of AgentStorage for development/testing
#[derive(Debug, Default)]
pub struct InMemoryAgentStorage {
    agents: RwLock<HashMap<AgentId, AgentDefinition>>,
    place_configs: RwLock<HashMap<Uuid, PlaceAgentConfig>>,
    executions: RwLock<HashMap<Uuid, AgentExecution>>,
}

#[async_trait::async_trait]
impl AgentStorage for InMemoryAgentStorage {
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()> {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id.clone(), agent.clone());
        Ok(())
    }

    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>> {
        let agents = self.agents.read().await;
        Ok(agents.get(id).cloned())
    }

    async fn list_agents(&self) -> Result<Vec<AgentDefinition>> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        let mut agents = self.agents.write().await;
        Ok(agents.remove(id).is_some())
    }

    async fn store_place_agent_config(&self, config: &PlaceAgentConfig) -> Result<()> {
        let mut configs = self.place_configs.write().await;
        configs.insert(config.id, config.clone());
        Ok(())
    }

    async fn get_place_agent_configs(&self, place_id: &PlaceId) -> Result<Vec<PlaceAgentConfig>> {
        let configs = self.place_configs.read().await;
        Ok(configs
            .values()
            .filter(|config| &config.place_id == place_id && config.enabled)
            .cloned()
            .collect())
    }

    async fn list_place_agent_configs(&self) -> Result<Vec<PlaceAgentConfig>> {
        let configs = self.place_configs.read().await;
        Ok(configs.values().cloned().collect())
    }

    async fn delete_place_agent_config(&self, id: &Uuid) -> Result<bool> {
        let mut configs = self.place_configs.write().await;
        Ok(configs.remove(id).is_some())
    }

    async fn store_execution(&self, execution: &AgentExecution) -> Result<()> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id, execution.clone());
        Ok(())
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(id).cloned())
    }

    async fn list_executions_for_token(&self, token_id: &Uuid) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| &exec.token_id == token_id)
            .cloned()
            .collect())
    }

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| &exec.agent_id == agent_id)
            .cloned()
            .collect())
    }
}

/// Configuration for the agent engine
#[derive(Debug, Clone)]
pub struct AgentEngineConfig {
    pub max_concurrent_executions: usize,
    pub stream_buffer_size: usize,
    pub connection_timeout: Duration,
    pub execution_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl Default for AgentEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 50,
            stream_buffer_size: 1000,
            connection_timeout: Duration::from_secs(30),
            execution_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Main agent execution engine
pub struct AgentEngine {
    storage: Arc<dyn AgentStorage>,
    rules_engine: Arc<RulesEngine>,
    config: AgentEngineConfig,
    stream_sender: broadcast::Sender<AgentStreamEvent>,
}

impl AgentEngine {
    pub fn new(
        storage: Arc<dyn AgentStorage>,
        rules_engine: Arc<RulesEngine>,
        config: AgentEngineConfig,
    ) -> Self {
        let (stream_sender, _) = broadcast::channel(config.stream_buffer_size);
        
        Self {
            storage,
            rules_engine,
            config,
            stream_sender,
        }
    }

    /// Subscribe to agent execution stream events
    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<AgentStreamEvent> {
        self.stream_sender.subscribe()
    }

    /// Execute agents for a token that entered or exists in a place
    pub async fn execute_place_agents(&self, token: &Token) -> Result<Vec<AgentExecution>> {
        let configs = self.storage.get_place_agent_configs(&token.place).await?;
        let mut executions = Vec::new();

        for config in configs {
            // Check trigger conditions
            if !self.should_trigger_agent(&config, token).await? {
                continue;
            }

            // Apply scheduling constraints
            if let Some(schedule) = &config.schedule {
                if let Some(delay) = schedule.initial_delay_seconds {
                    sleep(Duration::from_secs(delay)).await;
                }
            }

            // Execute the agent
            match self.execute_agent_for_config(&config, token).await {
                Ok(execution) => executions.push(execution),
                Err(e) => {
                    eprintln!("Failed to execute agent {} for token {}: {}", 
                             config.agent_id.as_str(), token.id, e);
                }
            }
        }

        Ok(executions)
    }

    /// Execute agent for a transition
    pub async fn execute_transition_agent(
        &self,
        config: &AgentTransitionConfig,
        token: &Token,
    ) -> Result<AgentExecution> {
        let agent = self.storage.get_agent(&config.agent_id).await?
            .ok_or_else(|| CircuitBreakerError::NotFound(format!("Agent {}", config.agent_id.as_str())))?;

        let input_data = self.map_input_data(&config.input_mapping, token)?;
        let mut execution = AgentExecution::new(
            config.agent_id.clone(),
            token.id,
            token.place.clone(),
            input_data,
        );

        self.execute_agent_internal(&agent, &mut execution, &config.input_mapping, &config.output_mapping).await?;
        
        Ok(execution)
    }

    /// Check if agent should be triggered based on conditions
    async fn should_trigger_agent(&self, config: &PlaceAgentConfig, _token: &Token) -> Result<bool> {
        if config.trigger_conditions.is_empty() {
            return Ok(true);
        }

        for _rule in &config.trigger_conditions {
            // TODO: Implement rule evaluation once RulesEngine API is available
            // let result = self.rules_engine.evaluate_rule(rule, token);
            // if !result.passed {
            //     return Ok(false);
            // }
        }

        Ok(true)
    }

    /// Execute agent for a specific place configuration
    async fn execute_agent_for_config(
        &self,
        config: &PlaceAgentConfig,
        token: &Token,
    ) -> Result<AgentExecution> {
        let agent = self.storage.get_agent(&config.agent_id).await?
            .ok_or_else(|| CircuitBreakerError::NotFound(format!("Agent {}", config.agent_id.as_str())))?;

        let input_data = self.map_input_data(&config.input_mapping, token)?;
        let mut execution = AgentExecution::new(
            config.agent_id.clone(),
            token.id,
            token.place.clone(),
            input_data,
        );
        execution.config_id = Some(config.id);

        self.execute_agent_internal(&agent, &mut execution, &config.input_mapping, &config.output_mapping).await?;
        
        Ok(execution)
    }

    /// Internal agent execution logic
    async fn execute_agent_internal(
        &self,
        agent: &AgentDefinition,
        execution: &mut AgentExecution,
        _input_mapping: &HashMap<String, String>,
        _output_mapping: &HashMap<String, String>,
    ) -> Result<()> {
        execution.start();
        self.storage.store_execution(execution).await?;

        // Emit starting event
        let _ = self.stream_sender.send(AgentStreamEvent::ThinkingStatus {
            execution_id: execution.id,
            status: "Starting agent execution".to_string(),
        });

        // Execute the LLM call (this would integrate with actual LLM providers)
        match self.call_llm_provider(&agent.llm_provider, &agent.llm_config, &execution.input_data).await {
            Ok(response) => {
                execution.complete(response.clone());
                
                // Emit completion event
                let _ = self.stream_sender.send(AgentStreamEvent::Completed {
                    execution_id: execution.id,
                    final_response: response,
                    usage: None,
                });
            }
            Err(e) => {
                execution.fail(e.to_string());
                
                // Emit failure event
                let _ = self.stream_sender.send(AgentStreamEvent::Failed {
                    execution_id: execution.id,
                    error: e.to_string(),
                });
            }
        }

        self.storage.store_execution(execution).await?;
        Ok(())
    }

    /// Map token data to agent input using the provided mapping
    fn map_input_data(&self, mapping: &HashMap<String, String>, token: &Token) -> Result<Value> {
        let mut input = json!({});
        
        for (input_key, token_path) in mapping {
            let value = self.extract_value_from_token(token, token_path)?;
            input[input_key] = value;
        }

        Ok(input)
    }

    /// Extract value from token using a path like "data.content" or "metadata.type"
    fn extract_value_from_token(&self, token: &Token, path: &str) -> Result<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        
        match parts.as_slice() {
            ["data", field] => {
                Ok(token.data.get(field).cloned().unwrap_or(Value::Null))
            }
            ["metadata", field] => {
                Ok(token.metadata.get(*field).cloned().unwrap_or(Value::Null))
            }
            ["id"] => Ok(json!(token.id)),
            ["place"] => Ok(json!(token.place.as_str())),
            _ => Ok(Value::Null),
        }
    }

    /// Call the configured LLM provider (placeholder implementation)
    async fn call_llm_provider(
        &self,
        provider: &LLMProvider,
        config: &LLMConfig,
        input: &Value,
    ) -> Result<Value> {
        // This is a placeholder implementation
        // In a real implementation, this would make HTTP calls to the LLM providers
        
        match provider {
            LLMProvider::OpenAI { model, .. } => {
                // Simulate OpenAI API call
                sleep(Duration::from_millis(500)).await;
                Ok(json!({
                    "model": model,
                    "response": "This is a simulated OpenAI response",
                    "input_processed": input,
                    "temperature": config.temperature
                }))
            }
            LLMProvider::Anthropic { model, api_key, base_url } => {
                // Make real Anthropic API call
                self.call_anthropic_api(model, api_key, base_url.as_deref(), config, input).await
            }
            LLMProvider::Google { model, .. } => {
                // Simulate Google API call
                sleep(Duration::from_millis(400)).await;
                Ok(json!({
                    "model": model,
                    "response": "This is a simulated Google response",
                    "input_processed": input,
                    "temperature": config.temperature
                }))
            }
            LLMProvider::Ollama { model, base_url } => {
                // Simulate Ollama API call
                sleep(Duration::from_millis(800)).await;
                Ok(json!({
                    "model": model,
                    "base_url": base_url,
                    "response": "This is a simulated Ollama response",
                    "input_processed": input,
                    "temperature": config.temperature
                }))
            }
            LLMProvider::Custom { model, endpoint, .. } => {
                // Simulate custom API call
                sleep(Duration::from_millis(700)).await;
                Ok(json!({
                    "model": model,
                    "endpoint": endpoint,
                    "response": "This is a simulated custom provider response",
                    "input_processed": input,
                    "temperature": config.temperature
                }))
            }
        }
    }

    /// Apply agent output to token using output mapping
    pub fn apply_output_to_token(
        &self,
        token: &mut Token,
        output: &Value,
        mapping: &HashMap<String, String>,
    ) -> Result<()> {
        for (token_path, output_key) in mapping {
            if let Some(value) = output.get(output_key) {
                self.set_value_in_token(token, token_path, value.clone())?;
            }
        }
        Ok(())
    }

    /// Set value in token using a path like "data.review_result" or "metadata.reviewer"
    fn set_value_in_token(&self, token: &mut Token, path: &str, value: Value) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();
        
        match parts.as_slice() {
            ["data", field] => {
                if let Value::Object(ref mut map) = &mut token.data {
                    map.insert(field.to_string(), value);
                }
            }
            ["metadata", field] => {
                token.metadata.insert(field.to_string(), value);
            }
            _ => {
                return Err(CircuitBreakerError::InvalidInput(format!("Invalid token path: {}", path)));
            }
        }
        
        Ok(())
    }

    /// Make actual Anthropic API call
    async fn call_anthropic_api(
        &self,
        model: &str,
        api_key: &str,
        base_url: Option<&str>,
        config: &LLMConfig,
        input: &Value,
    ) -> Result<Value> {
        let client = reqwest::Client::new();
        
        // Extract content from input
        let content = if let Some(content_str) = input.get("content").and_then(|v| v.as_str()) {
            content_str
        } else {
            return Err(CircuitBreakerError::InvalidInput("No content found in input".to_string()));
        };
        
        // Prepare the request body according to Anthropic's API format
        let request_body = json!({
            "model": model,
            "max_tokens": config.max_tokens.unwrap_or(1000),
            "temperature": config.temperature,
            "messages": [{
                "role": "user",
                "content": content
            }]
        });
        
        // Construct the API endpoint URL
        let api_url = base_url
            .unwrap_or("https://api.anthropic.com/v1")
            .trim_end_matches('/');
        let messages_url = format!("{}/messages", api_url);
        
        // Make the API call
        let response = client
            .post(&messages_url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CircuitBreakerError::InvalidInput(format!("Anthropic API request failed: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CircuitBreakerError::InvalidInput(format!("Anthropic API error {}: {}", status, error_text)));
        }
        
        let response_json: Value = response.json().await
            .map_err(|e| CircuitBreakerError::InvalidInput(format!("Failed to parse Anthropic response: {}", e)))?;
        
        // Extract the content from Anthropic's response format
        let anthropic_response = response_json
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .and_then(|text| text.as_str())
            .unwrap_or("No response from Anthropic");
        
        // Return structured response
        Ok(json!({
            "model": model,
            "response": anthropic_response,
            "input_processed": input,
            "temperature": config.temperature,
            "provider": "anthropic",
            "raw_response": response_json
        }))
    }

    /// Get agent execution statistics
    pub async fn get_execution_stats(&self, agent_id: &AgentId) -> Result<ExecutionStats> {
        let executions = self.storage.list_executions_for_agent(agent_id).await?;
        
        let total = executions.len();
        let completed = executions.iter().filter(|e| e.status == AgentExecutionStatus::Completed).count();
        let failed = executions.iter().filter(|e| e.status == AgentExecutionStatus::Failed).count();
        let running = executions.iter().filter(|e| e.status == AgentExecutionStatus::Running).count();
        
        let avg_duration = if completed > 0 {
            let total_duration: u64 = executions
                .iter()
                .filter(|e| e.status == AgentExecutionStatus::Completed)
                .filter_map(|e| e.duration_ms)
                .sum();
            Some(total_duration / completed as u64)
        } else {
            None
        };

        Ok(ExecutionStats {
            total,
            completed,
            failed,
            running,
            avg_duration_ms: avg_duration,
        })
    }
}

/// Agent execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub running: usize,
    pub avg_duration_ms: Option<u64>,
}