//! Circuit Breaker SDK Client
//!
//! This module provides the main client for communicating with the Circuit Breaker server.
//! It handles HTTP requests, GraphQL queries, and authentication.

use crate::{Error, Result};
use reqwest::{header, Client as HttpClient, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
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

/// Endpoint health status
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    pub graphql: bool,
    pub rest: bool,
    pub graphql_url: String,
    pub rest_url: String,
}

/// Main Circuit Breaker client
#[derive(Debug, Clone)]
pub struct Client {
    config: ClientConfig,
    http_client: Arc<HttpClient>,
    graphql_endpoint: String,
    rest_endpoint: String,
    endpoint_health: Option<EndpointHealth>,
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

        // Smart endpoint detection
        let (graphql_endpoint, rest_endpoint) = Self::determine_endpoints(&config.base_url)?;

        Ok(Self {
            config,
            http_client,
            graphql_endpoint,
            rest_endpoint,
            endpoint_health: None,
        })
    }

    /// Determine GraphQL and REST endpoints from base URL
    fn determine_endpoints(base_url: &Url) -> Result<(String, String)> {
        let port = base_url.port();
        let path = base_url.path();

        let (graphql_endpoint, rest_endpoint) = match port {
            Some(3000) => {
                // REST endpoint specified - port 3000 is our REST API
                let rest = base_url.to_string();
                let graphql = format!(
                    "{}://{}:4000/graphql",
                    base_url.scheme(),
                    base_url.host_str().unwrap_or("localhost")
                );
                (graphql, rest)
            }
            Some(4000) => {
                // GraphQL endpoint specified - port 4000 is our GraphQL API
                let graphql = if path.ends_with("graphql") {
                    base_url.to_string()
                } else {
                    format!("{}/graphql", base_url.as_str().trim_end_matches('/'))
                };
                let rest = format!(
                    "{}://{}:3000",
                    base_url.scheme(),
                    base_url.host_str().unwrap_or("localhost")
                );
                (graphql, rest)
            }
            _ => {
                // Check path to determine endpoint type, or use defaults
                if path.contains("graphql") {
                    // GraphQL endpoint specified via path
                    let graphql = if path.ends_with("graphql") {
                        base_url.to_string()
                    } else {
                        format!("{}/graphql", base_url.as_str().trim_end_matches('/'))
                    };
                    let rest = format!(
                        "{}://{}:3000",
                        base_url.scheme(),
                        base_url.host_str().unwrap_or("localhost")
                    );
                    (graphql, rest)
                } else {
                    // Default ports
                    let graphql = format!(
                        "{}://{}:4000/graphql",
                        base_url.scheme(),
                        base_url.host_str().unwrap_or("localhost")
                    );
                    let rest = format!(
                        "{}://{}:3000",
                        base_url.scheme(),
                        base_url.host_str().unwrap_or("localhost")
                    );
                    (graphql, rest)
                }
            }
        };

        // Debug logging to show determined endpoints
        tracing::debug!(
            "Determined endpoints from base URL '{}': GraphQL='{}', REST='{}'",
            base_url,
            graphql_endpoint,
            rest_endpoint
        );

        Ok((graphql_endpoint, rest_endpoint))
    }

    /// Create a client builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Test connection to both REST and GraphQL endpoints
    pub async fn ping(&self) -> Result<PingResponse> {
        // Check endpoint health first
        let health = self.check_endpoint_health().await?;

        if !health.graphql && !health.rest {
            return Err(Error::Network {
                message: "Neither GraphQL nor REST endpoints are available".to_string(),
            });
        }

        let mut status = "partial".to_string();
        let mut graphql_ok = false;
        let mut rest_ok = false;

        // Test GraphQL endpoint
        if health.graphql {
            let introspection_query = r#"
                query IntrospectionQuery {
                    __schema {
                        types {
                            name
                        }
                    }
                }
            "#;

            match self
                .graphql_request_raw::<serde_json::Value, serde_json::Value>(
                    introspection_query,
                    serde_json::Value::Null,
                )
                .await
            {
                Ok(_) => {
                    graphql_ok = true;
                    status = "ok".to_string();
                }
                Err(e) => {
                    eprintln!("GraphQL endpoint check failed: {}", e);
                }
            }
        }

        // Test REST endpoint
        if health.rest {
            let models_url = format!("{}/v1/models", self.rest_endpoint);
            match self.http_client.get(&models_url).send().await {
                Ok(response) if response.status().is_success() => {
                    rest_ok = true;
                    if status == "partial" {
                        status = "ok".to_string();
                    }
                }
                Ok(response) => {
                    eprintln!("REST endpoint returned status: {}", response.status());
                }
                Err(e) => {
                    eprintln!("REST endpoint check failed: {}", e);
                }
            }
        }

        Ok(PingResponse {
            status,
            version: "1.0.0".to_string(),
            uptime_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            endpoints: Some(EndpointStatus {
                graphql: graphql_ok,
                rest: rest_ok,
                graphql_url: health.graphql_url,
                rest_url: health.rest_url,
            }),
        })
    }

    /// Check health of both endpoints
    async fn check_endpoint_health(&self) -> Result<EndpointHealth> {
        let mut health = EndpointHealth {
            graphql: false,
            rest: false,
            graphql_url: self.graphql_endpoint.clone(),
            rest_url: self.rest_endpoint.clone(),
        };

        // Check GraphQL endpoint (GET should return method not allowed or similar)
        let graphql_check = self
            .http_client
            .get(&self.graphql_endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        health.graphql = match graphql_check {
            Ok(response) => {
                // GraphQL typically returns 405 (Method Not Allowed) or 400 for GET requests
                response.status().as_u16() == 405 || response.status().as_u16() == 400
            }
            Err(_) => false,
        };

        // Check REST endpoint
        let models_url = format!("{}/v1/models", self.rest_endpoint);
        let rest_check = self
            .http_client
            .get(&models_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        health.rest = match rest_check {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        };

        Ok(health)
    }

    /// Get server information from both endpoints
    pub async fn info(&self) -> Result<ServerInfo> {
        let health = self.check_endpoint_health().await?;

        let mut info = ServerInfo {
            name: "Circuit Breaker AI Workflow Engine".to_string(),
            version: "1.0.0".to_string(),
            features: Vec::new(),
            providers: Vec::new(),
            endpoints: Some(EndpointInfo {
                graphql: health.graphql,
                rest: health.rest,
            }),
        };

        // Get GraphQL schema info if available
        if health.graphql {
            let introspection_query = r#"
                query IntrospectionQuery {
                    __schema {
                        types {
                            name
                            kind
                        }
                    }
                }
            "#;

            if let Ok(result) = self
                .graphql_request_raw::<serde_json::Value, serde_json::Value>(
                    introspection_query,
                    serde_json::Value::Null,
                )
                .await
            {
                if let Some(types) = result
                    .get("__schema")
                    .and_then(|s| s.get("types"))
                    .and_then(|t| t.as_array())
                {
                    let mut features = Vec::new();
                    for type_obj in types {
                        if let Some(name) = type_obj.get("name").and_then(|n| n.as_str()) {
                            if name.contains("Workflow")
                                && !features.contains(&"workflows".to_string())
                            {
                                features.push("workflows".to_string());
                            }
                            if name.contains("Agent") && !features.contains(&"agents".to_string()) {
                                features.push("agents".to_string());
                            }
                            if name.contains("Rule") && !features.contains(&"rules".to_string()) {
                                features.push("rules".to_string());
                            }
                            if (name.contains("Llm") || name.contains("LLM"))
                                && !features.contains(&"llm".to_string())
                            {
                                features.push("llm".to_string());
                            }
                            if (name.contains("Mcp") || name.contains("MCP"))
                                && !features.contains(&"mcp".to_string())
                            {
                                features.push("mcp".to_string());
                            }
                            if name.contains("Analytics")
                                && !features.contains(&"analytics".to_string())
                            {
                                features.push("analytics".to_string());
                            }
                        }
                    }
                    info.features.extend(features);
                }
            }
        }

        // Get REST API info if available
        if health.rest {
            let models_url = format!("{}/v1/models", self.rest_endpoint);
            if let Ok(response) = self.http_client.get(&models_url).send().await {
                if response.status().is_success() {
                    if let Ok(models_data) = response.json::<serde_json::Value>().await {
                        if let Some(models_array) =
                            models_data.get("data").and_then(|d| d.as_array())
                        {
                            let providers: std::collections::HashSet<String> = models_array
                                .iter()
                                .filter_map(|model| {
                                    model
                                        .get("provider")
                                        .and_then(|p| p.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect();
                            info.providers = providers.into_iter().collect();
                            info.features.extend_from_slice(&[
                                "llm-routing".to_string(),
                                "smart-routing".to_string(),
                                "virtual-models".to_string(),
                                "streaming".to_string(),
                            ]);
                        }
                    }
                }
            }
        }

        Ok(info)
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

    /// Make a GraphQL request with endpoint validation
    pub async fn graphql<T, V>(&self, query: &str, variables: V) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        V: Serialize,
    {
        // Ensure GraphQL endpoint is available
        let health = self.check_endpoint_health().await?;
        if !health.graphql {
            return Err(Error::Network {
                message: "GraphQL endpoint is not available".to_string(),
            });
        }

        self.graphql_request_raw(query, variables).await
    }

    /// Make a raw GraphQL request without health checking
    async fn graphql_request_raw<T, V>(&self, query: &str, variables: V) -> Result<T>
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

        let response = self
            .http_client
            .post(&self.graphql_endpoint)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::Network {
                message: format!("GraphQL request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(Error::Server {
                status: response.status().as_u16(),
                message: format!("GraphQL server error: {}", response.status()),
            });
        }

        let graphql_response: GraphQLResponse<T> =
            response.json().await.map_err(|e| Error::Parse {
                message: format!("Failed to parse GraphQL response: {}", e),
            })?;

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

    /// Make a REST request with endpoint validation
    pub async fn rest<T, B>(&self, method: Method, path: &str, body: Option<B>) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        // Ensure REST endpoint is available
        let health = self.check_endpoint_health().await?;
        if !health.rest {
            return Err(Error::Network {
                message: "REST endpoint is not available".to_string(),
            });
        }

        let url = if path.starts_with('/') {
            format!("{}{}", self.rest_endpoint, path)
        } else {
            format!("{}/{}", self.rest_endpoint, path)
        };

        let mut request = self.http_client.request(method, &url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|e| Error::Network {
            message: format!("REST request failed: {}", e),
        })?;

        if response.status().is_success() {
            response.json().await.map_err(|e| Error::Parse {
                message: format!("Failed to parse REST response: {}", e),
            })
        } else {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            Err(Error::Server {
                status,
                message: error_text,
            })
        }
    }

    /// Smart request method that automatically routes to the appropriate endpoint
    pub async fn request<T, B>(&self, method: Method, path: &str, body: Option<B>) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        // Route OpenAI-compatible endpoints to REST
        if path.starts_with("/v1/") || path.starts_with("v1/") {
            return self.rest(method, path, body).await;
        }

        // Route GraphQL queries
        if path == "/graphql" || path == "graphql" {
            if method != Method::POST {
                return Err(Error::Configuration {
                    message: "GraphQL endpoint only supports POST requests".to_string(),
                });
            }

            // Extract query from body if it's a GraphQL request
            if let Some(ref graphql_body) = body {
                let body_value = serde_json::to_value(graphql_body).map_err(|e| Error::Parse {
                    message: format!("Failed to serialize GraphQL body: {}", e),
                })?;

                if let Some(query) = body_value.get("query").and_then(|q| q.as_str()) {
                    let variables = body_value
                        .get("variables")
                        .unwrap_or(&serde_json::Value::Null);
                    return self.graphql_request_raw(query, variables).await;
                }
            }
        }

        // Default to REST for other paths
        self.rest(method, path, body).await
    }

    /// Get the appropriate endpoint URL for a given operation
    pub fn get_endpoint_url(&self, operation: &str) -> &str {
        match operation {
            "graphql" => &self.graphql_endpoint,
            "rest" => &self.rest_endpoint,
            _ => &self.rest_endpoint,
        }
    }

    /// Check if a specific endpoint is available
    pub async fn is_endpoint_available(&self, endpoint: &str) -> Result<bool> {
        let health = self.check_endpoint_health().await?;
        Ok(match endpoint {
            "graphql" => health.graphql,
            "rest" => health.rest,
            _ => false,
        })
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
    pub endpoints: Option<EndpointStatus>,
}

/// Endpoint status information
#[derive(Debug, Deserialize)]
pub struct EndpointStatus {
    pub graphql: bool,
    pub rest: bool,
    pub graphql_url: String,
    pub rest_url: String,
}

/// Server information response
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub providers: Vec<String>,
    pub endpoints: Option<EndpointInfo>,
}

/// Endpoint information
#[derive(Debug, Deserialize)]
pub struct EndpointInfo {
    pub graphql: bool,
    pub rest: bool,
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
