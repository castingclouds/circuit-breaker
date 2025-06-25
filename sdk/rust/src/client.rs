//! Circuit Breaker SDK Client
//!
//! This module provides the main client for communicating with the Circuit Breaker server.
//! It handles HTTP requests, GraphQL queries, and authentication.

use crate::{schema::QueryBuilder, Error, Result};
use reqwest::{header, Client as HttpClient, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// Configuration for the Circuit Breaker client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: Url,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub user_agent: String,
    pub headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: Url::parse(crate::DEFAULT_BASE_URL).unwrap(),
            api_key: None,
            timeout_ms: 30000,
            user_agent: format!("circuit-breaker-sdk-rust/{}", crate::VERSION),
            headers: HashMap::new(),
        }
    }
}

/// Main Circuit Breaker client
#[derive(Debug, Clone)]
pub struct Client {
    config: ClientConfig,
    http_client: Arc<HttpClient>,
}

impl Client {
    /// Create a new client with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        let mut headers = header::HeaderMap::new();

        // Add user agent
        headers.insert(
            header::USER_AGENT,
            config
                .user_agent
                .parse()
                .map_err(|e| Error::Configuration {
                    message: format!("Invalid user agent: {}", e),
                })?,
        );

        // Add API key if provided
        if let Some(api_key) = &config.api_key {
            headers.insert(
                header::AUTHORIZATION,
                format!("Bearer {}", api_key)
                    .parse()
                    .map_err(|e| Error::Configuration {
                        message: format!("Invalid API key: {}", e),
                    })?,
            );
        }

        // Add custom headers
        for (key, value) in &config.headers {
            let header_name = header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
                Error::Configuration {
                    message: format!("Invalid header name '{}': {}", key, e),
                }
            })?;
            let header_value = value.parse().map_err(|e| Error::Configuration {
                message: format!("Invalid header value for '{}': {}", key, e),
            })?;
            headers.insert(header_name, header_value);
        }

        let http_client = Arc::new(
            HttpClient::builder()
                .timeout(std::time::Duration::from_millis(config.timeout_ms))
                .default_headers(headers)
                .build()
                .map_err(|e| Error::Configuration {
                    message: format!("Failed to create HTTP client: {}", e),
                })?,
        );

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create a client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Test connection to the server
    pub async fn ping(&self) -> Result<PingResponse> {
        let query = QueryBuilder::query(
            "Ping",
            "llmProviders",
            &["name", "healthStatus { isHealthy }"],
        );

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmProviders")]
            llm_providers: Vec<LlmProviderHealth>,
        }

        #[derive(Deserialize)]
        struct LlmProviderHealth {
            name: String,
            #[serde(rename = "healthStatus")]
            health_status: HealthStatus,
        }

        #[derive(Deserialize)]
        struct HealthStatus {
            #[serde(rename = "isHealthy")]
            is_healthy: bool,
        }

        let result: Response = self.graphql(&query, ()).await?;

        let _healthy_providers = result
            .llm_providers
            .iter()
            .filter(|provider| provider.health_status.is_healthy)
            .count();

        Ok(PingResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(), // Default version
            uptime_seconds: 0,            // Not available from GraphQL
        })
    }

    /// Get server information
    pub async fn info(&self) -> Result<ServerInfo> {
        let query = QueryBuilder::query(
            "Info",
            "llmProviders",
            &["name", "healthStatus { isHealthy }"],
        );

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmProviders")]
            llm_providers: Vec<LlmProviderInfo>,
        }

        #[derive(Deserialize)]
        struct LlmProviderInfo {
            name: String,
            #[serde(rename = "healthStatus")]
            health_status: HealthStatus,
        }

        #[derive(Deserialize)]
        struct HealthStatus {
            #[serde(rename = "isHealthy")]
            is_healthy: bool,
        }

        let result: Response = self.graphql(&query, ()).await?;

        let features: Vec<String> = result
            .llm_providers
            .iter()
            .map(|p| format!("llm-{}", p.name.to_lowercase()))
            .collect();

        Ok(ServerInfo {
            name: "Circuit Breaker".to_string(),
            version: "1.0.0".to_string(), // Default version
            features,
        })
    }

    /// Access workflows API
    pub fn workflows(&self) -> crate::workflows::WorkflowClient {
        crate::workflows::WorkflowClient::new(self.clone())
    }

    /// Access agents API
    pub fn agents(&self) -> crate::agents::AgentClient {
        crate::agents::AgentClient::new(self.clone())
    }

    /// Access functions API
    pub fn functions(&self) -> crate::functions::FunctionClient {
        crate::functions::FunctionClient::new(self.clone())
    }

    /// Access resources API
    pub fn resources(&self) -> crate::resources::ResourceClient {
        crate::resources::ResourceClient::new(self.clone())
    }

    /// Access rules API
    pub fn rules(&self) -> crate::rules::RuleClient {
        crate::rules::RuleClient::new(self.clone())
    }

    /// Access LLM API
    pub fn llm(&self) -> crate::llm::LLMClient {
        crate::llm::LLMClient::new(self.clone())
    }

    /// Access analytics and budget management API
    pub fn analytics(&self) -> crate::analytics::AnalyticsClient {
        crate::analytics::AnalyticsClient::new(self.clone())
    }

    /// Access MCP (Model Context Protocol) API
    pub fn mcp(&self) -> crate::mcp::MCPClient {
        crate::mcp::MCPClient::new(self.clone())
    }

    /// Access NATS-enhanced operations API
    pub fn nats(&self) -> crate::nats::NATSClient {
        crate::nats::NATSClient::new(self.clone())
    }

    /// Access real-time subscription API
    pub fn subscriptions(&self) -> crate::subscriptions::SubscriptionClient {
        crate::subscriptions::SubscriptionClient::new(self.clone())
    }

    /// Get the base URL for the client
    pub fn base_url(&self) -> &url::Url {
        &self.config.base_url
    }

    /// Get the API key for the client
    pub fn api_key(&self) -> Option<&str> {
        self.config.api_key.as_deref()
    }

    /// Get the timeout in milliseconds
    pub fn timeout_ms(&self) -> u64 {
        self.config.timeout_ms
    }

    /// Make a GraphQL request
    pub async fn graphql<T, V>(&self, query: &str, variables: V) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        V: Serialize,
    {
        #[derive(Serialize)]
        struct GraphQLRequest<V> {
            query: String,
            variables: V,
        }

        #[derive(Deserialize)]
        struct GraphQLResponse<T> {
            data: Option<T>,
            errors: Option<Vec<GraphQLError>>,
        }

        #[derive(Deserialize)]
        struct GraphQLError {
            message: String,
        }

        let request_body = GraphQLRequest {
            query: query.to_string(),
            variables,
        };

        let url = self.config.base_url.join("/graphql")?;
        let response = self
            .http_client
            .post(url)
            .json(&request_body)
            .send()
            .await?;

        let graphql_response: GraphQLResponse<T> = response.json().await?;

        if let Some(errors) = graphql_response.errors {
            let error_messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(Error::Server {
                status: 400,
                message: format!("GraphQL errors: {}", error_messages.join(", ")),
            });
        }

        graphql_response.data.ok_or_else(|| Error::Parse {
            message: "GraphQL response missing data field".to_string(),
        })
    }

    /// Make a GraphQL query with optional variables
    pub async fn graphql_query<T, V>(&self, query: &str, variables: Option<V>) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        V: Serialize,
    {
        match variables {
            Some(vars) => self.graphql(&query, vars).await,
            None => self.graphql(&query, serde_json::Value::Null).await,
        }
    }

    /// Make a REST request
    pub async fn rest<T, B>(&self, method: Method, path: &str, body: Option<B>) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        let url = self.config.base_url.join(path)?;
        let mut request = self.http_client.request(method, url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            Err(Error::Server {
                status,
                message: error_text,
            })
        }
    }

    /// Get the HTTP client
    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }
}

/// Builder for creating a Circuit Breaker client
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    /// Set the base URL
    pub fn base_url(mut self, base_url: &str) -> Result<Self> {
        self.config.base_url = Url::parse(base_url)?;
        Ok(self)
    }

    /// Set the API key
    pub fn api_key(mut self, api_key: String) -> Self {
        self.config.api_key = Some(api_key);
        self
    }

    /// Set the timeout
    pub fn timeout(mut self, timeout_ms: u64) -> Self {
        self.config.timeout_ms = timeout_ms;
        self
    }

    /// Add a custom header
    pub fn header(mut self, key: String, value: String) -> Self {
        self.config.headers.insert(key, value);
        self
    }

    /// Build the client
    pub fn build(self) -> Result<Client> {
        Client::new(self.config)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from the ping endpoint
#[derive(Debug, Deserialize)]
pub struct PingResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Server information response
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .unwrap()
            .api_key("test-key".to_string())
            .timeout(60000)
            .header("X-Custom".to_string(), "value".to_string())
            .build()
            .unwrap();

        assert_eq!(client.config.base_url.as_str(), "https://api.example.com/");
        assert_eq!(client.config.api_key, Some("test-key".to_string()));
        assert_eq!(client.config.timeout_ms, 60000);
    }
}
