//! Model Context Protocol (MCP) Client
//!
//! This module provides functionality for managing MCP servers, handling OAuth/JWT authentication,
//! and managing sessions for the Circuit Breaker workflow automation server.
//!
//! # Examples
//!
//! ```rust
//! use circuit_breaker_sdk::{Client, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = Client::builder()
//!         .base_url("http://localhost:4000")?
//!         .build()?;
//!
//!     // List all MCP servers
//!     let servers = client.mcp()
//!         .servers()
//!         .list()
//!         .await?;
//!
//!     println!("Found {} MCP servers", servers.servers.len());
//!
//!     // Create a new MCP server
//!     let server = client.mcp()
//!         .create_server()
//!         .name("My MCP Server")
//!         .server_type("custom")
//!         .config(serde_json::json!({"endpoint": "http://localhost:8080"}))
//!         .execute()
//!         .await?;
//!
//!     // Configure OAuth for the server
//!     let oauth_config = client.mcp()
//!         .configure_oauth()
//!         .server_id(&server.id)
//!         .provider("google")
//!         .client_id("your-client-id")
//!         .client_secret("your-client-secret")
//!         .execute()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

use crate::client::Client;
use crate::types::*;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP client for Model Context Protocol operations
pub struct MCPClient {
    client: Client,
}

impl MCPClient {
    /// Create a new MCP client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get MCP servers with optional filtering
    pub fn servers(&self) -> MCPServersBuilder {
        MCPServersBuilder::new(self.client.clone())
    }

    /// Get a specific MCP server by ID
    pub async fn get_server(&self, id: &str) -> Result<Option<MCPServer>> {
        let query = r#"
            query GetMCPServer($id: ID!) {
                mcpServer(id: $id) {
                    id
                    name
                    description
                    type
                    status
                    config
                    capabilities
                    health
                    tenantId
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "mcpServer")]
            mcp_server: Option<MCPServerGQL>,
        }

        let variables = Variables { id: id.to_string() };
        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.mcp_server.map(|s| s.into()))
    }

    /// Create a new MCP server
    pub fn create_server(&self) -> CreateMCPServerBuilder {
        CreateMCPServerBuilder::new(self.client.clone())
    }

    /// Update an existing MCP server
    pub fn update_server(&self, id: &str) -> UpdateMCPServerBuilder {
        UpdateMCPServerBuilder::new(self.client.clone(), id.to_string())
    }

    /// Delete an MCP server
    pub async fn delete_server(&self, id: &str) -> Result<ApiResponse> {
        let query = r#"
            mutation DeleteMCPServer($id: ID!) {
                deleteMcpServer(id: $id) {
                    success
                    message
                    errorCode
                    data
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "deleteMcpServer")]
            delete_mcp_server: ApiResponse,
        }

        let variables = Variables { id: id.to_string() };
        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.delete_mcp_server)
    }

    /// Configure OAuth for an MCP server
    pub fn configure_oauth(&self) -> ConfigureOAuthBuilder {
        ConfigureOAuthBuilder::new(self.client.clone())
    }

    /// Configure JWT authentication for an MCP server
    pub fn configure_jwt(&self) -> ConfigureJWTBuilder {
        ConfigureJWTBuilder::new(self.client.clone())
    }

    /// Get available OAuth providers
    pub async fn get_oauth_providers(&self) -> Result<Vec<MCPOAuthProvider>> {
        let query = r#"
            query GetMCPOAuthProviders {
                mcpOAuthProviders {
                    id
                    name
                    type
                    config
                    isEnabled
                }
            }
        "#;

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "mcpOAuthProviders")]
            mcp_oauth_providers: Vec<MCPOAuthProviderGQL>,
        }

        let response: Response = self.client.graphql_query(query, None::<()>).await?;

        Ok(response
            .mcp_oauth_providers
            .into_iter()
            .map(|p| p.into())
            .collect())
    }

    /// Get server capabilities
    pub async fn get_server_capabilities(
        &self,
        server_id: &str,
    ) -> Result<Option<MCPServerCapabilities>> {
        let query = r#"
            query GetMCPServerCapabilities($serverId: ID!) {
                mcpServerCapabilities(serverId: $serverId) {
                    tools
                    resources
                    prompts
                    sampling
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "serverId")]
            server_id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "mcpServerCapabilities")]
            mcp_server_capabilities: Option<MCPServerCapabilitiesGQL>,
        }

        let variables = Variables {
            server_id: server_id.to_string(),
        };
        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.mcp_server_capabilities.map(|c| c.into()))
    }

    /// Get server health status
    pub async fn get_server_health(&self, server_id: &str) -> Result<MCPServerHealth> {
        let query = r#"
            query GetMCPServerHealth($serverId: ID!) {
                mcpServerHealth(serverId: $serverId) {
                    status
                    message
                    lastCheck
                    responseTime
                    details
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "serverId")]
            server_id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "mcpServerHealth")]
            mcp_server_health: MCPServerHealthGQL,
        }

        let variables = Variables {
            server_id: server_id.to_string(),
        };
        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.mcp_server_health.into())
    }

    /// Initiate OAuth flow
    pub async fn initiate_oauth(
        &self,
        server_id: &str,
        user_id: &str,
    ) -> Result<MCPOAuthInitiation> {
        let query = r#"
            mutation InitiateMCPOAuth($input: InitiateMcpOAuthInput!) {
                initiateMcpOAuth(input: $input) {
                    authUrl
                    state
                    codeChallenge
                    expiresAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            #[serde(rename = "serverId")]
            server_id: String,
            #[serde(rename = "userId")]
            user_id: String,
        }

        #[derive(Serialize)]
        struct Variables {
            input: Input,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "initiateMcpOAuth")]
            initiate_mcp_oauth: MCPOAuthInitiationGQL,
        }

        let variables = Variables {
            input: Input {
                server_id: server_id.to_string(),
                user_id: user_id.to_string(),
            },
        };

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.initiate_mcp_oauth.into())
    }

    /// Complete OAuth flow
    pub async fn complete_oauth(&self, state: &str, code: &str) -> Result<MCPSession> {
        let query = r#"
            mutation CompleteMCPOAuth($input: CompleteMcpOAuthInput!) {
                completeMcpOAuth(input: $input) {
                    id
                    serverId
                    userId
                    status
                    accessToken
                    refreshToken
                    expiresAt
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            state: String,
            code: String,
        }

        #[derive(Serialize)]
        struct Variables {
            input: Input,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "completeMcpOAuth")]
            complete_mcp_oauth: MCPSessionGQL,
        }

        let variables = Variables {
            input: Input {
                state: state.to_string(),
                code: code.to_string(),
            },
        };

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.complete_mcp_oauth.into())
    }
}

/// MCP Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServer {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub server_type: MCPServerType,
    pub status: MCPServerStatus,
    pub config: serde_json::Value,
    pub capabilities: Option<serde_json::Value>,
    pub health: Option<serde_json::Value>,
    pub tenant_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP Server connection wrapper
#[derive(Debug, Clone, Deserialize)]
pub struct MCPServerConnection {
    pub servers: Vec<MCPServer>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

/// MCP Server type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPServerType {
    #[serde(rename = "BUILT_IN")]
    BuiltIn,
    #[serde(rename = "CUSTOM")]
    Custom,
    #[serde(rename = "THIRD_PARTY")]
    ThirdParty,
}

/// MCP Server status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPServerStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "INACTIVE")]
    Inactive,
    #[serde(rename = "ERROR")]
    Error,
    #[serde(rename = "CONNECTING")]
    Connecting,
}

/// OAuth provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPOAuthProvider {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub config: serde_json::Value,
    pub is_enabled: bool,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerCapabilities {
    pub tools: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
    pub prompts: Option<Vec<String>>,
    pub sampling: Option<serde_json::Value>,
}

/// Server health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerHealth {
    pub status: String,
    pub message: Option<String>,
    pub last_check: Option<String>,
    pub response_time: Option<i32>,
    pub details: Option<serde_json::Value>,
}

/// OAuth initiation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPOAuthInitiation {
    pub auth_url: String,
    pub state: String,
    pub code_challenge: Option<String>,
    pub expires_at: String,
}

/// MCP Session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSession {
    pub id: String,
    pub server_id: String,
    pub user_id: String,
    pub status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// GraphQL response types
#[derive(Deserialize)]
struct MCPServerGQL {
    id: String,
    name: String,
    description: Option<String>,
    #[serde(rename = "type")]
    server_type: MCPServerType,
    status: MCPServerStatus,
    config: serde_json::Value,
    capabilities: Option<serde_json::Value>,
    health: Option<serde_json::Value>,
    #[serde(rename = "tenantId")]
    tenant_id: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl From<MCPServerGQL> for MCPServer {
    fn from(gql: MCPServerGQL) -> Self {
        Self {
            id: gql.id,
            name: gql.name,
            description: gql.description,
            server_type: gql.server_type,
            status: gql.status,
            config: gql.config,
            capabilities: gql.capabilities,
            health: gql.health,
            tenant_id: gql.tenant_id,
            created_at: gql.created_at,
            updated_at: gql.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct MCPOAuthProviderGQL {
    id: String,
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    config: serde_json::Value,
    #[serde(rename = "isEnabled")]
    is_enabled: bool,
}

impl From<MCPOAuthProviderGQL> for MCPOAuthProvider {
    fn from(gql: MCPOAuthProviderGQL) -> Self {
        Self {
            id: gql.id,
            name: gql.name,
            provider_type: gql.provider_type,
            config: gql.config,
            is_enabled: gql.is_enabled,
        }
    }
}

#[derive(Deserialize)]
struct MCPServerCapabilitiesGQL {
    tools: Option<Vec<String>>,
    resources: Option<Vec<String>>,
    prompts: Option<Vec<String>>,
    sampling: Option<serde_json::Value>,
}

impl From<MCPServerCapabilitiesGQL> for MCPServerCapabilities {
    fn from(gql: MCPServerCapabilitiesGQL) -> Self {
        Self {
            tools: gql.tools,
            resources: gql.resources,
            prompts: gql.prompts,
            sampling: gql.sampling,
        }
    }
}

#[derive(Deserialize)]
struct MCPServerHealthGQL {
    status: String,
    message: Option<String>,
    #[serde(rename = "lastCheck")]
    last_check: Option<String>,
    #[serde(rename = "responseTime")]
    response_time: Option<i32>,
    details: Option<serde_json::Value>,
}

impl From<MCPServerHealthGQL> for MCPServerHealth {
    fn from(gql: MCPServerHealthGQL) -> Self {
        Self {
            status: gql.status,
            message: gql.message,
            last_check: gql.last_check,
            response_time: gql.response_time,
            details: gql.details,
        }
    }
}

#[derive(Deserialize)]
struct MCPOAuthInitiationGQL {
    #[serde(rename = "authUrl")]
    auth_url: String,
    state: String,
    #[serde(rename = "codeChallenge")]
    code_challenge: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: String,
}

impl From<MCPOAuthInitiationGQL> for MCPOAuthInitiation {
    fn from(gql: MCPOAuthInitiationGQL) -> Self {
        Self {
            auth_url: gql.auth_url,
            state: gql.state,
            code_challenge: gql.code_challenge,
            expires_at: gql.expires_at,
        }
    }
}

#[derive(Deserialize)]
struct MCPSessionGQL {
    id: String,
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    status: String,
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl From<MCPSessionGQL> for MCPSession {
    fn from(gql: MCPSessionGQL) -> Self {
        Self {
            id: gql.id,
            server_id: gql.server_id,
            user_id: gql.user_id,
            status: gql.status,
            access_token: gql.access_token,
            refresh_token: gql.refresh_token,
            expires_at: gql.expires_at,
            created_at: gql.created_at,
            updated_at: gql.updated_at,
        }
    }
}

/// Builder for MCP servers queries
pub struct MCPServersBuilder {
    client: Client,
    server_type: Option<MCPServerType>,
    status: Option<MCPServerStatus>,
    tenant_id: Option<String>,
    pagination: Option<PaginationInput>,
}

impl MCPServersBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            server_type: None,
            status: None,
            tenant_id: None,
            pagination: None,
        }
    }

    /// Filter by server type
    pub fn server_type(mut self, server_type: MCPServerType) -> Self {
        self.server_type = Some(server_type);
        self
    }

    /// Filter by server status
    pub fn status(mut self, status: MCPServerStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by tenant ID
    pub fn tenant_id<S: Into<String>>(mut self, tenant_id: S) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Set pagination parameters
    pub fn pagination(mut self, pagination: PaginationInput) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// Execute the query
    pub async fn list(self) -> Result<MCPServerConnection> {
        let query = if self.tenant_id.is_some() {
            r#"
                query GetMCPServersByTenant($tenantId: String!, $pagination: PaginationInput) {
                    mcpServersByTenant(tenantId: $tenantId, pagination: $pagination) {
                        servers {
                            id
                            name
                            description
                            type
                            status
                            config
                            capabilities
                            health
                            tenantId
                            createdAt
                            updatedAt
                        }
                        pageInfo {
                            hasNextPage
                            hasPreviousPage
                            startCursor
                            endCursor
                            totalCount
                        }
                        totalCount
                    }
                }
            "#
        } else {
            r#"
                query GetMCPServers($type: McpServerType, $status: McpServerStatus, $pagination: PaginationInput) {
                    mcpServers(type: $type, status: $status, pagination: $pagination) {
                        servers {
                            id
                            name
                            description
                            type
                            status
                            config
                            capabilities
                            health
                            tenantId
                            createdAt
                            updatedAt
                        }
                        pageInfo {
                            hasNextPage
                            hasPreviousPage
                            startCursor
                            endCursor
                            totalCount
                        }
                        totalCount
                    }
                }
            "#
        };

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "type")]
            server_type: Option<MCPServerType>,
            status: Option<MCPServerStatus>,
            #[serde(rename = "tenantId")]
            tenant_id: Option<String>,
            pagination: Option<PaginationInput>,
        }

        let variables = Variables {
            server_type: self.server_type,
            status: self.status,
            tenant_id: self.tenant_id.clone(),
            pagination: self.pagination,
        };

        if self.tenant_id.is_some() {
            #[derive(Deserialize)]
            struct Response {
                #[serde(rename = "mcpServersByTenant")]
                mcp_servers_by_tenant: MCPServerConnectionGQL,
            }

            let response: Response = self.client.graphql_query(query, Some(variables)).await?;
            Ok(response.mcp_servers_by_tenant.into())
        } else {
            #[derive(Deserialize)]
            struct Response {
                #[serde(rename = "mcpServers")]
                mcp_servers: MCPServerConnectionGQL,
            }

            let response: Response = self.client.graphql_query(query, Some(variables)).await?;
            Ok(response.mcp_servers.into())
        }
    }
}

#[derive(Deserialize)]
struct MCPServerConnectionGQL {
    servers: Vec<MCPServerGQL>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    #[serde(rename = "totalCount")]
    total_count: i32,
}

impl From<MCPServerConnectionGQL> for MCPServerConnection {
    fn from(gql: MCPServerConnectionGQL) -> Self {
        Self {
            servers: gql.servers.into_iter().map(|s| s.into()).collect(),
            page_info: gql.page_info,
            total_count: gql.total_count,
        }
    }
}

/// Builder for creating MCP servers
pub struct CreateMCPServerBuilder {
    client: Client,
    name: Option<String>,
    description: Option<String>,
    server_type: Option<MCPServerType>,
    config: Option<serde_json::Value>,
    tenant_id: Option<String>,
}

impl CreateMCPServerBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            name: None,
            description: None,
            server_type: None,
            config: None,
            tenant_id: None,
        }
    }

    /// Set the server name
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the server description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the server type
    pub fn server_type(mut self, server_type: MCPServerType) -> Self {
        self.server_type = Some(server_type);
        self
    }

    /// Set the server configuration
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the tenant ID
    pub fn tenant_id<S: Into<String>>(mut self, tenant_id: S) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Execute the create server mutation
    pub async fn execute(self) -> Result<MCPServer> {
        let query = r#"
            mutation CreateMCPServer($input: CreateMcpServerInput!) {
                createMcpServer(input: $input) {
                    id
                    name
                    description
                    type
                    status
                    config
                    capabilities
                    health
                    tenantId
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            name: String,
            description: Option<String>,
            #[serde(rename = "type")]
            server_type: MCPServerType,
            config: serde_json::Value,
            #[serde(rename = "tenantId")]
            tenant_id: Option<String>,
        }

        #[derive(Serialize)]
        struct Variables {
            input: Input,
        }

        let input = Input {
            name: self.name.ok_or_else(|| crate::Error::Validation {
                message: "name is required".to_string(),
            })?,
            description: self.description,
            server_type: self.server_type.ok_or_else(|| crate::Error::Validation {
                message: "server_type is required".to_string(),
            })?,
            config: self.config.unwrap_or_else(|| serde_json::json!({})),
            tenant_id: self.tenant_id,
        };

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createMcpServer")]
            create_mcp_server: MCPServerGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.create_mcp_server.into())
    }
}

/// Builder for updating MCP servers
pub struct UpdateMCPServerBuilder {
    client: Client,
    id: String,
    name: Option<String>,
    description: Option<String>,
    status: Option<MCPServerStatus>,
    config: Option<serde_json::Value>,
}

impl UpdateMCPServerBuilder {
    fn new(client: Client, id: String) -> Self {
        Self {
            client,
            id,
            name: None,
            description: None,
            status: None,
            config: None,
        }
    }

    /// Set the server name
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the server description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the server status
    pub fn status(mut self, status: MCPServerStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the server configuration
    pub fn config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    /// Execute the update server mutation
    pub async fn execute(self) -> Result<MCPServer> {
        let query = r#"
            mutation UpdateMCPServer($id: ID!, $input: UpdateMcpServerInput!) {
                updateMcpServer(id: $id, input: $input) {
                    id
                    name
                    description
                    type
                    status
                    config
                    capabilities
                    health
                    tenantId
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            name: Option<String>,
            description: Option<String>,
            status: Option<MCPServerStatus>,
            config: Option<serde_json::Value>,
        }

        #[derive(Serialize)]
        struct Variables {
            id: String,
            input: Input,
        }

        let input = Input {
            name: self.name,
            description: self.description,
            status: self.status,
            config: self.config,
        };

        let variables = Variables { id: self.id, input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "updateMcpServer")]
            update_mcp_server: MCPServerGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.update_mcp_server.into())
    }
}

/// Builder for configuring OAuth
pub struct ConfigureOAuthBuilder {
    client: Client,
    server_id: Option<String>,
    provider: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    scopes: Option<Vec<String>>,
    redirect_uri: Option<String>,
}

impl ConfigureOAuthBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            server_id: None,
            provider: None,
            client_id: None,
            client_secret: None,
            scopes: None,
            redirect_uri: None,
        }
    }

    /// Set the server ID
    pub fn server_id<S: Into<String>>(mut self, server_id: S) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    /// Set the OAuth provider
    pub fn provider<S: Into<String>>(mut self, provider: S) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the client ID
    pub fn client_id<S: Into<String>>(mut self, client_id: S) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Set the client secret
    pub fn client_secret<S: Into<String>>(mut self, client_secret: S) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Set the OAuth scopes
    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = Some(scopes);
        self
    }

    /// Set the redirect URI
    pub fn redirect_uri<S: Into<String>>(mut self, redirect_uri: S) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    /// Execute the configure OAuth mutation
    pub async fn execute(self) -> Result<MCPOAuthConfig> {
        let query = r#"
            mutation ConfigureMCPOAuth($input: ConfigureMcpOAuthInput!) {
                configureMcpOAuth(input: $input) {
                    id
                    serverId
                    provider
                    clientId
                    scopes
                    redirectUri
                    isEnabled
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            #[serde(rename = "serverId")]
            server_id: String,
            provider: String,
            #[serde(rename = "clientId")]
            client_id: String,
            #[serde(rename = "clientSecret")]
            client_secret: String,
            scopes: Option<Vec<String>>,
            #[serde(rename = "redirectUri")]
            redirect_uri: Option<String>,
        }

        #[derive(Serialize)]
        struct Variables {
            input: Input,
        }

        let input = Input {
            server_id: self.server_id.ok_or_else(|| crate::Error::Validation {
                message: "server_id is required".to_string(),
            })?,
            provider: self.provider.ok_or_else(|| crate::Error::Validation {
                message: "provider is required".to_string(),
            })?,
            client_id: self.client_id.ok_or_else(|| crate::Error::Validation {
                message: "client_id is required".to_string(),
            })?,
            client_secret: self.client_secret.ok_or_else(|| crate::Error::Validation {
                message: "client_secret is required".to_string(),
            })?,
            scopes: self.scopes,
            redirect_uri: self.redirect_uri,
        };

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "configureMcpOAuth")]
            configure_mcp_oauth: MCPOAuthConfigGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.configure_mcp_oauth.into())
    }
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPOAuthConfig {
    pub id: String,
    pub server_id: String,
    pub provider: String,
    pub client_id: String,
    pub scopes: Option<Vec<String>>,
    pub redirect_uri: Option<String>,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
struct MCPOAuthConfigGQL {
    id: String,
    #[serde(rename = "serverId")]
    server_id: String,
    provider: String,
    #[serde(rename = "clientId")]
    client_id: String,
    scopes: Option<Vec<String>>,
    #[serde(rename = "redirectUri")]
    redirect_uri: Option<String>,
    #[serde(rename = "isEnabled")]
    is_enabled: bool,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl From<MCPOAuthConfigGQL> for MCPOAuthConfig {
    fn from(gql: MCPOAuthConfigGQL) -> Self {
        Self {
            id: gql.id,
            server_id: gql.server_id,
            provider: gql.provider,
            client_id: gql.client_id,
            scopes: gql.scopes,
            redirect_uri: gql.redirect_uri,
            is_enabled: gql.is_enabled,
            created_at: gql.created_at,
            updated_at: gql.updated_at,
        }
    }
}

/// Builder for configuring JWT
pub struct ConfigureJWTBuilder {
    client: Client,
    server_id: Option<String>,
    secret_key: Option<String>,
    algorithm: Option<String>,
    expiration: Option<i32>,
}

impl ConfigureJWTBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            server_id: None,
            secret_key: None,
            algorithm: None,
            expiration: None,
        }
    }

    /// Set the server ID
    pub fn server_id<S: Into<String>>(mut self, server_id: S) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    /// Set the JWT secret key
    pub fn secret_key<S: Into<String>>(mut self, secret_key: S) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    /// Set the JWT algorithm
    pub fn algorithm<S: Into<String>>(mut self, algorithm: S) -> Self {
        self.algorithm = Some(algorithm.into());
        self
    }

    /// Set the JWT expiration time (in seconds)
    pub fn expiration(mut self, expiration: i32) -> Self {
        self.expiration = Some(expiration);
        self
    }

    /// Execute the configure JWT mutation
    pub async fn execute(self) -> Result<MCPJWTConfig> {
        let query = r#"
            mutation ConfigureMCPJWT($input: ConfigureMcpJwtInput!) {
                configureMcpJwt(input: $input) {
                    id
                    serverId
                    algorithm
                    expiration
                    isEnabled
                    createdAt
                    updatedAt
                }
            }
        "#;

        #[derive(Serialize)]
        struct Input {
            #[serde(rename = "serverId")]
            server_id: String,
            #[serde(rename = "secretKey")]
            secret_key: String,
            algorithm: String,
            expiration: i32,
        }

        #[derive(Serialize)]
        struct Variables {
            input: Input,
        }

        let input = Input {
            server_id: self.server_id.ok_or_else(|| crate::Error::Validation {
                message: "server_id is required".to_string(),
            })?,
            secret_key: self.secret_key.ok_or_else(|| crate::Error::Validation {
                message: "secret_key is required".to_string(),
            })?,
            algorithm: self.algorithm.unwrap_or_else(|| "HS256".to_string()),
            expiration: self.expiration.unwrap_or(3600), // Default 1 hour
        };

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "configureMcpJwt")]
            configure_mcp_jwt: MCPJWTConfigGQL,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(response.configure_mcp_jwt.into())
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPJWTConfig {
    pub id: String,
    pub server_id: String,
    pub algorithm: String,
    pub expiration: i32,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
struct MCPJWTConfigGQL {
    id: String,
    #[serde(rename = "serverId")]
    server_id: String,
    algorithm: String,
    expiration: i32,
    #[serde(rename = "isEnabled")]
    is_enabled: bool,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

impl From<MCPJWTConfigGQL> for MCPJWTConfig {
    fn from(gql: MCPJWTConfigGQL) -> Self {
        Self {
            id: gql.id,
            server_id: gql.server_id,
            algorithm: gql.algorithm,
            expiration: gql.expiration,
            is_enabled: gql.is_enabled,
            created_at: gql.created_at,
            updated_at: gql.updated_at,
        }
    }
}

/// Convenience functions
pub fn create_mcp_server(
    client: &Client,
    name: &str,
    server_type: MCPServerType,
) -> CreateMCPServerBuilder {
    client
        .mcp()
        .create_server()
        .name(name)
        .server_type(server_type)
}

pub fn list_mcp_servers(client: &Client) -> MCPServersBuilder {
    client.mcp().servers()
}

pub async fn get_mcp_server_health(client: &Client, server_id: &str) -> Result<MCPServerHealth> {
    client.mcp().get_server_health(server_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_type_serialization() {
        let server_type = MCPServerType::Custom;
        let json = serde_json::to_string(&server_type).unwrap();
        assert_eq!(json, "\"CUSTOM\"");

        let deserialized: MCPServerType = serde_json::from_str(&json).unwrap();
        matches!(deserialized, MCPServerType::Custom);
    }

    #[test]
    fn test_mcp_server_status_serialization() {
        let status = MCPServerStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ACTIVE\"");

        let deserialized: MCPServerStatus = serde_json::from_str(&json).unwrap();
        matches!(deserialized, MCPServerStatus::Active);
    }
}
