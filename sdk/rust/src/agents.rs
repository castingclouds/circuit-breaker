//! Agents module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for creating and managing AI agents.

use crate::{schema::QueryBuilder, types::*, ChatMessage, ChatRole, Client, Result};
use serde::{Deserialize, Serialize};

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

        // Use the agent's LLM configuration to send the message via llmChatCompletion
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
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmChatCompletion")]
            llm_chat_completion: crate::llm::ChatCompletionResponse,
        }

        // Create messages including the agent's system prompt
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: self.data.prompts.system.clone(),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: message,
                name: None,
            },
        ];

        let response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    input: LlmchatCompletionInput {
                        model: self.data.llm_provider.model.clone(),
                        messages,
                        temperature: Some(self.data.llm_config.temperature as f32),
                        max_tokens: self.data.llm_config.max_tokens.map(|t| t as u32),
                    },
                },
            )
            .await?;

        // Extract the assistant's response
        if let Some(choice) = response.llm_chat_completion.choices.first() {
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
