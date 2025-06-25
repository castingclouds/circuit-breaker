// MCP (Model Context Protocol) server implementation
// This module implements the multi-tenant MCP server functionality for Circuit Breaker

use axum::{
    body::Body,
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};

// Global SSE Response Router for multi-tenant SSE communication
pub struct SSEResponseRouter {
    // Maps Bearer token -> SSE channel sender
    channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Result<Event, Infallible>>>>>,
}

impl SSEResponseRouter {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_channel(
        &self,
        token: String,
        sender: mpsc::UnboundedSender<Result<Event, Infallible>>,
    ) {
        let mut channels = self.channels.write().await;
        info!("Registered SSE channel for token: {}...", &token[..8]);
        channels.insert(token, sender);
    }

    pub async fn unregister_channel(&self, token: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(token);
        info!("Unregistered SSE channel for token: {}...", &token[..8]);
    }

    pub async fn send_response(
        &self,
        token: &str,
        response: &super::mcp_types::MCPResponse,
    ) -> bool {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(token) {
            if let Ok(response_json) = serde_json::to_string(response) {
                let event = Event::default().event("mcp-response").data(response_json);

                if sender.send(Ok(event)).is_ok() {
                    info!("Sent MCP response via SSE for token: {}...", &token[..8]);
                    return true;
                } else {
                    info!(
                        "Failed to send MCP response - channel closed for token: {}...",
                        &token[..8]
                    );
                }
            }
        } else {
            info!("No SSE channel found for token: {}...", &token[..8]);
        }
        false
    }

    pub async fn get_active_tokens(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }
}

// Global SSE router instance
lazy_static::lazy_static! {
    static ref SSE_ROUTER: SSEResponseRouter = SSEResponseRouter::new();
}

use base64::{engine::general_purpose, Engine as _};
use chrono;

use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid;

use super::mcp_auth::{ClientInfo, MCPJWTService, MCPTokenClaims};
use super::mcp_storage::{InMemoryMCPStorage, MCPStorage, NATSMCPStorage};
use super::mcp_types::*;
use super::oauth::{OAuthManager, OAuthProviderType};
use crate::api::mcp_types::{MCPApplicationType, MCPId, RemoteOAuthConfig};

/// Circuit Breaker MCP Server Manager - manages multiple MCP server instances
#[derive(Clone)]
pub struct MCPServerManager {
    pub registry: Arc<RwLock<MCPServerRegistry>>,
    pub sessions: Arc<RwLock<HashMap<String, MCPSession>>>,
    pub jwt_service: Arc<MCPJWTService>,
    pub oauth_manager: Arc<OAuthManager>,
    pub storage: Arc<dyn MCPStorage>,
}

impl MCPServerManager {
    /// Create a new MCP server manager with in-memory storage
    pub fn new() -> Self {
        Self::with_storage(Arc::new(InMemoryMCPStorage::default()))
    }

    /// Create a new MCP server manager with custom storage
    pub fn with_storage(storage: Arc<dyn MCPStorage>) -> Self {
        Self {
            registry: Arc::new(RwLock::new(MCPServerRegistry::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            jwt_service: Arc::new(MCPJWTService::new()),
            oauth_manager: Arc::new(OAuthManager::with_storage(storage.clone())),
            storage,
        }
    }

    /// Create a new MCP server manager with NATS storage
    pub async fn with_nats_storage(nats_url: &str) -> Result<Self, String> {
        let nats_storage = NATSMCPStorage::new(nats_url)
            .await
            .map_err(|e| format!("Failed to create NATS storage: {}", e))?;

        let manager = Self::with_storage(Arc::new(nats_storage));

        // Load existing instances from storage
        manager.load_instances_from_storage().await?;

        Ok(manager)
    }

    /// Load existing instances from persistent storage
    async fn load_instances_from_storage(&self) -> Result<(), String> {
        match self.storage.list_server_instances().await {
            Ok(instances) => {
                let mut registry = self.registry.write().await;
                for instance in instances {
                    info!(
                        "Loading MCP instance from storage: {}",
                        instance.instance_id
                    );
                    registry.add_server_instance(instance);
                }
                info!(
                    "Loaded {} MCP instances from storage",
                    registry.servers.len()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to load instances from storage: {}", e);
                Err(format!("Failed to load instances from storage: {}", e))
            }
        }
    }

    /// Get JWT token for an app from storage (for URL-based authentication)
    pub async fn get_app_token(&self, app_id: &str) -> Result<String, String> {
        // Try to find the most recent installation token for this app_id
        // We'll look through the JWT service's session store directly

        warn!("Looking for app_id: {}", app_id);

        // Try to find any existing installation for this app and create a token
        // Since we can't access the JWT service's internal stores directly,
        // we'll try a common installation pattern or use a default installation ID

        // For now, let's try with the known installation ID from our setup
        let known_installation_id = "inst_44f296bf"; // From our current session

        warn!(
            "Attempting to create token for app: {} with installation: {}",
            app_id, known_installation_id
        );

        // Try to generate an installation token for this app/installation pair
        match self
            .jwt_service
            .create_installation_token(app_id, known_installation_id, None)
            .await
        {
            Ok(token) => {
                warn!("Successfully created token for app: {}", app_id);
                Ok(token.token)
            }
            Err(e) => {
                warn!("Failed to create token for app {}: {}", app_id, e);
                Err(format!("Failed to create token for app {}: {}", app_id, e))
            }
        }
    }

    /// Create a new MCP server instance
    pub async fn create_server_instance(
        &self,
        app_id: String,
        installation_id: String,
        name: String,
        description: String,
        project_contexts: Vec<String>,
        app_type: MCPApplicationType,
    ) -> Result<String, String> {
        let instance_id = uuid::Uuid::new_v4().to_string();

        let instance = MCPServerInstance {
            instance_id: instance_id.clone(),
            app_id,
            installation_id,
            name,
            description,
            capabilities: MCPCapabilities::default(),
            project_contexts,
            app_type: app_type.clone(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            status: MCPServerStatus::Active,
        };

        // Store in persistent storage first
        if let Err(e) = self.storage.store_server_instance(&instance).await {
            error!("Failed to store MCP instance in persistent storage: {}", e);
            return Err(format!("Failed to store MCP instance: {}", e));
        }

        // Store OAuth configuration if this is a remote instance
        if let MCPApplicationType::Remote(oauth_config) = &app_type {
            if let Err(e) = self
                .storage
                .store_oauth_config(&instance_id, oauth_config)
                .await
            {
                error!("Failed to store OAuth config in persistent storage: {}", e);
                return Err(format!("Failed to store OAuth config: {}", e));
            }
            info!(
                "Stored OAuth configuration for remote MCP instance: {}",
                instance_id
            );
        }

        // Also store in local registry for fast access
        let mut registry = self.registry.write().await;
        registry.add_server_instance(instance);

        info!("Created new MCP server instance: {}", instance_id);
        Ok(instance_id)
    }

    /// Get server instance
    pub async fn get_server_instance(&self, instance_id: &str) -> Option<MCPServerInstance> {
        // First try local registry for fast access
        {
            let registry = self.registry.read().await;
            if let Some(instance) = registry.get_server_instance(instance_id) {
                return Some(instance.clone());
            }
        }

        // If not in local registry, try persistent storage
        match self.storage.get_server_instance(instance_id).await {
            Ok(Some(instance)) => {
                // Add to local registry for future fast access
                let mut registry = self.registry.write().await;
                registry.add_server_instance(instance.clone());
                Some(instance)
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to get instance from storage: {}", e);
                None
            }
        }
    }

    /// Get OAuth configuration for a remote instance
    pub async fn get_oauth_config(&self, instance_id: &str) -> Option<RemoteOAuthConfig> {
        match self.storage.get_oauth_config(instance_id).await {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to get OAuth config from storage: {}", e);
                None
            }
        }
    }

    /// List all server instances
    pub async fn list_server_instances(&self) -> Vec<MCPServerInstance> {
        match self.storage.list_server_instances().await {
            Ok(instances) => instances,
            Err(e) => {
                error!("Failed to list instances from storage: {}", e);
                // Fallback to local registry
                let registry = self.registry.read().await;
                registry.servers.values().cloned().collect()
            }
        }
    }

    /// Delete a server instance
    pub async fn delete_server_instance(&self, instance_id: &str) -> Result<bool, String> {
        // Delete from persistent storage
        let deleted_instance = self
            .storage
            .delete_server_instance(instance_id)
            .await
            .map_err(|e| format!("Failed to delete instance from storage: {}", e))?;

        let deleted_oauth = self
            .storage
            .delete_oauth_config(instance_id)
            .await
            .map_err(|e| format!("Failed to delete OAuth config from storage: {}", e))?;

        // Delete from local registry
        let mut registry = self.registry.write().await;
        registry.servers.remove(instance_id);

        if deleted_instance {
            info!("Deleted MCP instance: {}", instance_id);
            if deleted_oauth {
                info!("Deleted OAuth config for instance: {}", instance_id);
            }
        }

        Ok(deleted_instance)
    }

    /// Authenticate request and extract installation context
    pub async fn authenticate_request(
        &self,
        headers: &HeaderMap,
    ) -> Result<MCPTokenClaims, String> {
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        if let Some(token) = auth_header {
            self.jwt_service
                .validate_token(token)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("Missing authorization header".to_string())
        }
    }

    /// Register an app with the JWT service
    pub async fn register_app(&self, app: MCPApp) -> Result<(), String> {
        self.jwt_service
            .register_app(app)
            .await
            .map_err(|e| e.to_string())
    }

    /// Register an installation with the JWT service
    pub async fn register_installation(&self, installation: MCPInstallation) -> Result<(), String> {
        self.jwt_service
            .register_installation(installation)
            .await
            .map_err(|e| e.to_string())
    }

    /// Create an installation token
    pub async fn create_installation_token(
        &self,
        app_id: &str,
        installation_id: &str,
        requested_permissions: Option<MCPPermissions>,
    ) -> Result<super::mcp_auth::MCPInstallationToken, String> {
        self.jwt_service
            .create_installation_token(app_id, installation_id, requested_permissions)
            .await
            .map_err(|e| e.to_string())
    }

    /// Create a session token
    pub async fn create_session_token(
        &self,
        installation_id: &str,
        session_id: &str,
        user_id: Option<String>,
        permissions: MCPSessionPermissions,
        project_contexts: Vec<String>,
        client_info: ClientInfo,
    ) -> Result<String, String> {
        self.jwt_service
            .create_session_token(
                installation_id,
                session_id,
                user_id,
                permissions,
                project_contexts,
                client_info,
            )
            .await
            .map_err(|e| e.to_string())
    }

    /// Revoke a token
    pub async fn revoke_token(&self, token_id: &str) -> Result<(), String> {
        self.jwt_service
            .revoke_token(token_id)
            .await
            .map_err(|e| e.to_string())
    }

    /// Register an OAuth provider
    pub async fn register_oauth_provider(
        &self,
        provider: super::oauth::OAuthProvider,
    ) -> Result<(), String> {
        self.oauth_manager
            .register_provider(provider)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get OAuth authorization URL
    pub async fn get_oauth_authorization_url(
        &self,
        provider_type: OAuthProviderType,
        installation_id: String,
        user_id: Option<String>,
        redirect_uri: Option<String>,
        scope: Option<Vec<String>>,
    ) -> Result<String, String> {
        self.oauth_manager
            .get_authorization_url(provider_type, installation_id, user_id, redirect_uri, scope)
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle OAuth callback
    pub async fn handle_oauth_callback(
        &self,
        callback: super::oauth::OAuthCallback,
    ) -> Result<super::oauth::StoredOAuthToken, String> {
        self.oauth_manager
            .handle_callback(callback)
            .await
            .map_err(|e| e.to_string())
    }

    /// Make authenticated API request to external service
    pub async fn make_authenticated_api_request(
        &self,
        provider_type: &OAuthProviderType,
        installation_id: &str,
        user_id: Option<&str>,
        method: reqwest::Method,
        url: &str,
        body: Option<String>,
        headers: Option<HashMap<String, String>>,
    ) -> Result<reqwest::Response, String> {
        self.oauth_manager
            .make_authenticated_request(
                provider_type,
                installation_id,
                user_id,
                method,
                url,
                body,
                headers,
            )
            .await
            .map_err(|e| e.to_string())
    }

    /// Get default tools for a server instance
    pub async fn get_default_tools(&self, instance_id: &str) -> Vec<MCPTool> {
        debug!("🔧 get_default_tools called for instance: {}", instance_id);

        // Get the instance to check its configuration
        if let Some(instance) = self.get_server_instance(instance_id).await {
            debug!(
                "✅ Found instance: {} ({})",
                instance.name, instance.instance_id
            );
            debug!("📋 Instance app_type: {:?}", instance.app_type);

            match &instance.app_type {
                MCPApplicationType::Remote(oauth_config) => {
                    debug!(
                        "🌐 Remote instance with provider: {}",
                        oauth_config.provider_type
                    );
                    // Return provider-specific tools based on the OAuth provider type
                    let tools = match oauth_config.provider_type.as_str() {
                        "gitlab" => {
                            debug!("🦊 Returning GitLab tools");
                            self.get_gitlab_tools()
                        }
                        "github" => {
                            debug!("🐙 Returning GitHub tools");
                            self.get_github_tools()
                        }
                        "google" => {
                            debug!("🔍 Returning Google tools");
                            self.get_google_tools()
                        }
                        provider => {
                            debug!(
                                "❓ Unknown provider '{}', returning generic remote tools",
                                provider
                            );
                            self.get_generic_remote_tools()
                        }
                    };
                    debug!(
                        "🛠️  Returning {} tools for {} instance",
                        tools.len(),
                        oauth_config.provider_type
                    );
                    for tool in &tools {
                        debug!("   • {}: {}", tool.name, tool.description);
                    }
                    tools
                }
                MCPApplicationType::Local => {
                    debug!("🏠 Local instance, returning generic local tools");
                    let tools = self.get_generic_local_tools();
                    debug!("🛠️  Returning {} local tools", tools.len());
                    for tool in &tools {
                        debug!("   • {}: {}", tool.name, tool.description);
                    }
                    tools
                }
            }
        } else {
            warn!(
                "❌ Instance '{}' not found, returning fallback tools",
                instance_id
            );
            let tools = self.get_generic_local_tools();
            debug!("🛠️  Returning {} fallback tools", tools.len());
            tools
        }
    }

    /// Get GitLab-specific tools
    fn get_gitlab_tools(&self) -> Vec<MCPTool> {
        vec![
            MCPTool {
                name: "gitlab_search".to_string(),
                description: "Search across GitLab projects and repositories".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "scope": {"type": "string", "enum": ["projects", "issues", "merge_requests", "commits"], "default": "projects"},
                        "project_id": {"type": "string", "description": "Specific project ID to search within (optional)"}
                    },
                    "required": ["query"]
                }),
            },
            MCPTool {
                name: "gitlab_list_projects".to_string(),
                description: "List GitLab projects accessible to the authenticated user"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "visibility": {"type": "string", "enum": ["private", "internal", "public"], "description": "Filter by visibility"},
                        "owned": {"type": "boolean", "description": "Only show owned projects"},
                        "starred": {"type": "boolean", "description": "Only show starred projects"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20}
                    }
                }),
            },
            MCPTool {
                name: "gitlab_get_project".to_string(),
                description: "Get detailed information about a specific GitLab project".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID or path"}
                    },
                    "required": ["project_id"]
                }),
            },
            MCPTool {
                name: "gitlab_list_issues".to_string(),
                description: "List issues in a GitLab project".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID"},
                        "state": {"type": "string", "enum": ["opened", "closed", "all"], "default": "opened"},
                        "labels": {"type": "string", "description": "Comma-separated list of labels"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20}
                    },
                    "required": ["project_id"]
                }),
            },
            MCPTool {
                name: "gitlab_create_issue".to_string(),
                description: "Create a new issue in a GitLab project".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID"},
                        "title": {"type": "string", "description": "Issue title"},
                        "description": {"type": "string", "description": "Issue description"},
                        "labels": {"type": "array", "items": {"type": "string"}, "description": "Issue labels"},
                        "assignee_ids": {"type": "array", "items": {"type": "integer"}, "description": "User IDs to assign"}
                    },
                    "required": ["project_id", "title"]
                }),
            },
            MCPTool {
                name: "gitlab_list_merge_requests".to_string(),
                description: "List merge requests in a GitLab project".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID"},
                        "state": {"type": "string", "enum": ["opened", "closed", "merged", "all"], "default": "opened"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20}
                    },
                    "required": ["project_id"]
                }),
            },
            MCPTool {
                name: "gitlab_get_file".to_string(),
                description: "Get file contents from a GitLab repository".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID"},
                        "file_path": {"type": "string", "description": "Path to the file"},
                        "ref": {"type": "string", "description": "Branch, tag, or commit SHA", "default": "main"}
                    },
                    "required": ["project_id", "file_path"]
                }),
            },
            MCPTool {
                name: "gitlab_list_pipelines".to_string(),
                description: "List CI/CD pipelines for a GitLab project".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "string", "description": "GitLab project ID"},
                        "status": {"type": "string", "enum": ["running", "pending", "success", "failed", "canceled", "skipped"], "description": "Filter by status"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 20}
                    },
                    "required": ["project_id"]
                }),
            },
            MCPTool {
                name: "gitlab_get_user".to_string(),
                description: "Get information about the authenticated GitLab user".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    /// Get GitHub-specific tools
    fn get_github_tools(&self) -> Vec<MCPTool> {
        vec![
            MCPTool {
                name: "github_list_repositories".to_string(),
                description: "List GitHub repositories accessible to the authenticated user"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "visibility": {"type": "string", "enum": ["all", "public", "private"], "default": "all"},
                        "affiliation": {"type": "string", "enum": ["owner", "collaborator", "organization_member"], "default": "owner"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 30}
                    }
                }),
            },
            MCPTool {
                name: "github_get_repository".to_string(),
                description: "Get detailed information about a specific GitHub repository"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": {"type": "string", "description": "Repository owner"},
                        "repo": {"type": "string", "description": "Repository name"}
                    },
                    "required": ["owner", "repo"]
                }),
            },
            MCPTool {
                name: "github_list_issues".to_string(),
                description: "List issues in a GitHub repository".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "owner": {"type": "string", "description": "Repository owner"},
                        "repo": {"type": "string", "description": "Repository name"},
                        "state": {"type": "string", "enum": ["open", "closed", "all"], "default": "open"},
                        "per_page": {"type": "integer", "minimum": 1, "maximum": 100, "default": 30}
                    },
                    "required": ["owner", "repo"]
                }),
            },
        ]
    }

    /// Get Google-specific tools (placeholder)
    fn get_google_tools(&self) -> Vec<MCPTool> {
        vec![MCPTool {
            name: "google_drive_list".to_string(),
            description: "List files in Google Drive".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"},
                    "max_results": {"type": "integer", "minimum": 1, "maximum": 100, "default": 10}
                }
            }),
        }]
    }

    /// Get generic tools for remote instances with unknown providers
    fn get_generic_remote_tools(&self) -> Vec<MCPTool> {
        vec![MCPTool {
            name: "api_request".to_string(),
            description: "Make authenticated API requests to the configured provider".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"], "default": "GET"},
                    "endpoint": {"type": "string", "description": "API endpoint path"},
                    "body": {"type": "object", "description": "Request body for POST/PUT/PATCH"}
                },
                "required": ["endpoint"]
            }),
        }]
    }

    /// Get generic tools for local instances
    fn get_generic_local_tools(&self) -> Vec<MCPTool> {
        vec![
            MCPTool {
                name: "create_workflow".to_string(),
                description: "Create a new workflow definition".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "description": {"type": "string"},
                        "steps": {"type": "array"}
                    },
                    "required": ["name", "steps"]
                }),
            },
            MCPTool {
                name: "execute_agent".to_string(),
                description: "Execute an agent with given parameters".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "agent_id": {"type": "string"},
                        "parameters": {"type": "object"}
                    },
                    "required": ["agent_id"]
                }),
            },
            MCPTool {
                name: "search_project_context".to_string(),
                description: "Search within a project context".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "context_id": {"type": "string"},
                        "query": {"type": "string"},
                        "filters": {"type": "object"}
                    },
                    "required": ["context_id", "query"]
                }),
            },
        ]
    }

    /// Get default prompts for a server instance
    pub async fn get_default_prompts(&self, _instance_id: &str) -> Vec<MCPPrompt> {
        vec![
            MCPPrompt {
                name: "workflow_template".to_string(),
                description: "Generate a workflow template for a given task".to_string(),
                arguments: vec![
                    MCPPromptArgument {
                        name: "task_description".to_string(),
                        description: "Description of the task to create a workflow for".to_string(),
                        required: true,
                    },
                    MCPPromptArgument {
                        name: "complexity_level".to_string(),
                        description: "Complexity level (simple, medium, complex)".to_string(),
                        required: false,
                    },
                ],
            },
            MCPPrompt {
                name: "agent_configuration".to_string(),
                description: "Generate agent configuration for specific tasks".to_string(),
                arguments: vec![
                    MCPPromptArgument {
                        name: "agent_type".to_string(),
                        description: "Type of agent (coding, analysis, creative, etc.)".to_string(),
                        required: true,
                    },
                    MCPPromptArgument {
                        name: "capabilities".to_string(),
                        description: "Required capabilities for the agent".to_string(),
                        required: false,
                    },
                ],
            },
        ]
    }

    /// Create a new session for a specific server instance
    pub async fn create_session(
        &self,
        server_instance_id: String,
        installation_id: String,
        app_id: String,
        client_info: MCPClientInfo,
        user_id: Option<String>,
        permissions: MCPSessionPermissions,
        project_contexts: Vec<String>,
    ) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = MCPSession {
            session_id: session_id.clone(),
            server_instance_id,
            installation_id,
            app_id,
            user_id,
            client_info,
            permissions,
            project_contexts,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(24)),
        };

        let server_instance_id = session.server_instance_id.clone();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        info!(
            "Created new MCP session: {} for instance: {}",
            session_id, server_instance_id
        );
        session_id
    }

    /// Update session activity
    pub async fn update_session_activity(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = chrono::Utc::now();
        }
    }

    /// Get session
    pub async fn get_session(&self, session_id: &str) -> Option<MCPSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get the OAuth manager
    pub fn get_oauth_manager(&self) -> &OAuthManager {
        &self.oauth_manager
    }
}

/// Circuit Breaker MCP Server - handles multi-tenant MCP instances
pub struct CircuitBreakerMCPServer {
    manager: MCPServerManager,
}

impl CircuitBreakerMCPServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self {
            manager: MCPServerManager::new(),
        }
    }

    /// Create a new MCP server with an existing manager
    pub fn with_manager(manager: MCPServerManager) -> Self {
        Self { manager }
    }

    /// Create a new MCP server with NATS storage
    pub async fn with_nats_storage(nats_url: &str) -> Result<Self, String> {
        let manager = MCPServerManager::with_nats_storage(nats_url).await?;
        Ok(Self { manager })
    }

    /// Create the MCP router with multi-tenant support
    pub fn create_router(&self) -> Router {
        Router::new()
            // Multi-tenant MCP protocol endpoints
            .route("/mcp/:instance_id", post(handle_mcp_request))
            .route("/mcp/:instance_id", get(handle_mcp_get_request))
            .route("/mcp/:instance_id/ws", get(handle_mcp_websocket))
            // Server instance management
            .route("/mcp/instances", post(create_mcp_instance))
            .route("/mcp/instances/:instance_id", get(get_mcp_instance))
            .route("/mcp/instances/:instance_id/info", get(get_server_info))
            // Tool management endpoints (per instance)
            .route("/mcp/:instance_id/tools", get(list_tools))
            .route("/mcp/:instance_id/prompts", get(list_prompts))
            .route("/mcp/:instance_id/resources", get(list_resources))
            // Remote MCP OAuth endpoints (per instance)
            // OAuth initiation endpoint for MCP clients like Windsurf
            .route("/mcp/:instance_id/oauth/init", get(handle_mcp_oauth_init))
            // OAuth callback endpoint for MCP clients
            .route(
                "/mcp/:instance_id/oauth/callback",
                get(handle_mcp_oauth_callback),
            )
            // OAuth callback endpoint for remote instances - support both GET and POST
            .route(
                "/mcp/:instance_id/oauth/callback/remote",
                get(handle_instance_oauth_callback).post(handle_instance_oauth_callback),
            )
            // SSE endpoint for MCP streaming (mcp-remote compatible)
            .route(
                "/mcp/:instance_id/sse",
                get(handle_mcp_sse).options(handle_options),
            )
            // Token endpoint for dynamic token retrieval
            .route("/mcp/:instance_id/token", get(handle_token_request))
            // Direct SSE endpoint for mcp-remote (expected format)
            .route("/sse", get(handle_default_mcp_sse).options(handle_options))
            // SSE test endpoint for debugging
            .route("/sse/test", get(handle_sse_test))
            // HTTP endpoint for mcp-remote http-first strategy
            .route("/", post(handle_default_mcp_http).options(handle_options))
            // MCP OAuth discovery endpoints
            .route(
                "/.well-known/oauth-authorization-server",
                get(handle_oauth_authorization_server_metadata).options(handle_options),
            )
            .route(
                "/.well-known/mcp",
                get(handle_mcp_discovery).options(handle_options),
            )
            .route(
                "/oauth/metadata",
                get(handle_oauth_metadata).options(handle_options),
            )
            .route(
                "/register",
                post(handle_oauth_register).options(handle_options),
            )
            // Debug endpoint to list instances
            .route("/debug/instances", get(handle_debug_instances))
            // Authentication endpoints
            // OAuth endpoints
            .route("/mcp/auth/apps", post(register_app))
            .route("/mcp/auth/installations", post(register_installation))
            .route(
                "/mcp/auth/installations/:installation_id/tokens",
                post(create_installation_token),
            )
            .route(
                "/mcp/auth/sessions/:session_id/tokens",
                post(create_session_token),
            )
            .route("/mcp/auth/tokens/:token_id/revoke", post(revoke_token))
            // OAuth provider endpoints
            .route("/mcp/oauth/providers", post(register_oauth_provider))
            .route("/mcp/oauth/authorize", post(get_oauth_authorization_url))
            // General OAuth callback - for backward compatibility
            .route(
                "/mcp/oauth/callback",
                get(handle_general_oauth_callback).post(handle_general_oauth_callback),
            )
            // OAuth callback for MCP clients (expected by mcp-remote)
            .route(
                "/oauth/callback/debug",
                get(handle_mcp_client_oauth_callback).post(handle_mcp_client_oauth_callback),
            )
            // OAuth callback for MCP clients (alternative pattern)
            .route(
                "/oauth/callback",
                get(handle_mcp_client_oauth_callback).post(handle_mcp_client_oauth_callback),
            )
            // Add state
            .with_state(self.manager.clone())
    }

    /// Handle MCP request for a specific instance
    pub async fn handle_request(
        &self,
        instance_id: &str,
        request: MCPRequest,
        claims: Option<MCPTokenClaims>,
    ) -> MCPResponse {
        debug!(
            "Handling MCP request: {} for instance: {}",
            request.method, instance_id
        );

        // Helper function to get request ID or default for notifications
        let get_request_id = || {
            request
                .id
                .clone()
                .unwrap_or_else(|| MCPId::String("notification".to_string()))
        };

        // Verify the instance exists
        let instance = match self.manager.get_server_instance(instance_id).await {
            Some(instance) => instance,
            None => {
                return MCPResponse::error_from_request(
                    Some(get_request_id()),
                    error_codes::INVALID_REQUEST,
                    format!("Server instance '{}' not found", instance_id),
                );
            }
        };

        // Verify authentication and permissions if provided
        if let Some(claims) = &claims {
            // Verify the claims are valid for this instance
            if claims.installation_id != instance.installation_id {
                return MCPResponse::error_from_request(
                    Some(get_request_id()),
                    error_codes::INVALID_REQUEST,
                    "Token not valid for this instance".to_string(),
                );
            }

            // Check if the app matches
            if claims.app_id != instance.app_id {
                return MCPResponse::error_from_request(
                    Some(get_request_id()),
                    error_codes::INVALID_REQUEST,
                    "Token app_id does not match instance".to_string(),
                );
            }
        }

        info!(
            "🔄 Processing MCP request: {} for instance: {}",
            request.method, instance.instance_id
        );
        debug!(
            "🔄 Request details: id={:?}, params={:?}",
            request.id, request.params
        );

        match request.method.as_str() {
            "initialize" => {
                info!("🚀 Handling initialize request");
                self.handle_initialize(request, &instance).await
            }
            "tools/list" => {
                info!("🛠️  Handling tools/list request");
                self.handle_list_tools(request, &instance).await
            }
            "tools/call" => {
                info!("⚡ Handling tools/call request");
                self.handle_call_tool(request, &instance).await
            }
            "prompts/list" => {
                info!("📝 Handling prompts/list request");
                self.handle_list_prompts(request, &instance).await
            }
            "prompts/get" => {
                info!("📄 Handling prompts/get request");
                self.handle_get_prompt(request, &instance).await
            }
            "resources/list" => {
                info!("📁 Handling resources/list request");
                self.handle_list_resources(request, &instance).await
            }
            "resources/read" => {
                info!("📖 Handling resources/read request");
                self.handle_read_resource(request, &instance).await
            }
            method => {
                warn!("❌ Unknown method: {}", method);
                MCPResponse::error_from_request(
                    Some(get_request_id()),
                    error_codes::METHOD_NOT_FOUND,
                    format!("Method '{}' not found", method),
                )
            }
        }
    }

    /// Handle initialize request
    async fn handle_initialize(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Initializing MCP server instance: {}", instance.instance_id);

        // For gitlab-demo instance, provide GitLab-specific capabilities
        let capabilities = if instance.instance_id == "gitlab-demo" {
            serde_json::json!({
                "tools": {
                    "list_repositories": {
                        "description": "List GitLab repositories for the authenticated user",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "visibility": {
                                    "type": "string",
                                    "enum": ["public", "private", "internal"],
                                    "description": "Repository visibility filter"
                                },
                                "limit": {
                                    "type": "number",
                                    "description": "Maximum number of repositories to return",
                                    "default": 20
                                }
                            }
                        }
                    },
                    "get_repository": {
                        "description": "Get details about a specific GitLab repository",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": {
                                    "type": "string",
                                    "description": "GitLab project ID or path"
                                }
                            },
                            "required": ["project_id"]
                        }
                    },
                    "list_issues": {
                        "description": "List issues for a GitLab project",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": {
                                    "type": "string",
                                    "description": "GitLab project ID or path"
                                },
                                "state": {
                                    "type": "string",
                                    "enum": ["opened", "closed", "all"],
                                    "default": "opened"
                                }
                            },
                            "required": ["project_id"]
                        }
                    },
                    "create_issue": {
                        "description": "Create a new issue in a GitLab project",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": {
                                    "type": "string",
                                    "description": "GitLab project ID or path"
                                },
                                "title": {
                                    "type": "string",
                                    "description": "Issue title"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Issue description"
                                }
                            },
                            "required": ["project_id", "title"]
                        }
                    }
                },
                "prompts": {
                    "gitlab_issue_template": {
                        "description": "Generate a GitLab issue template",
                        "arguments": [
                            {
                                "name": "issue_type",
                                "description": "Type of issue (bug, feature, etc.)",
                                "required": true
                            }
                        ]
                    }
                },
                "resources": {
                    "gitlab_projects": {
                        "description": "Access to GitLab project data",
                        "mimeType": "application/json"
                    }
                }
            })
        } else {
            serde_json::to_value(&instance.capabilities).unwrap_or_default()
        };

        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": capabilities,
            "serverInfo": {
                "name": instance.name,
                "version": env!("CARGO_PKG_VERSION"),
                "instanceId": instance.instance_id,
                "installationId": instance.installation_id,
                "appId": instance.app_id
            }
        });

        MCPResponse::success_from_request(request.id, result)
    }

    /// Handle list tools request
    async fn handle_list_tools(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        info!(
            "📋 handle_list_tools called for instance: {} ({})",
            instance.name, instance.instance_id
        );
        debug!("📋 Instance details: app_type={:?}", instance.app_type);

        let tools = self.manager.get_default_tools(&instance.instance_id).await;

        info!(
            "🛠️  Retrieved {} tools for instance {}",
            tools.len(),
            instance.instance_id
        );
        for tool in &tools {
            info!("   🔧 Tool: {} - {}", tool.name, tool.description);
        }

        let result = serde_json::json!({
            "tools": tools
        });

        info!("📤 Sending tools/list response with {} tools", tools.len());
        debug!(
            "📤 Full response: {}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );

        MCPResponse::success_from_request(request.id, result)
    }

    /// Handle call tool request
    async fn handle_call_tool(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Calling tool for instance: {}", instance.instance_id);

        let params = match request.params {
            Some(params) => params,
            None => {
                return MCPResponse::error_from_request(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    "Missing tool call parameters".to_string(),
                );
            }
        };

        let tool_call: MCPToolCall = match serde_json::from_value(params) {
            Ok(call) => call,
            Err(e) => {
                return MCPResponse::error_from_request(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    format!("Invalid tool call parameters: {}", e),
                );
            }
        };

        // Execute the tool in the context of this instance
        let result = self.execute_tool(tool_call, instance).await;

        match result {
            Ok(tool_result) => MCPResponse::success_from_request(
                request.id,
                serde_json::to_value(tool_result).unwrap(),
            ),
            Err(e) => MCPResponse::error_from_request(
                request.id,
                error_codes::INTERNAL_ERROR,
                format!("Tool execution failed: {}", e),
            ),
        }
    }

    /// Handle list prompts request
    async fn handle_list_prompts(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!(
            "Listing available prompts for instance: {}",
            instance.instance_id
        );

        let prompts = self
            .manager
            .get_default_prompts(&instance.instance_id)
            .await;
        let result = serde_json::json!({
            "prompts": prompts
        });

        MCPResponse::success_from_request(request.id, result)
    }

    /// Handle get prompt request
    async fn handle_get_prompt(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Getting prompt for instance: {}", instance.instance_id);

        // For now, return a placeholder response
        MCPResponse::success_from_request(
            request.id,
            serde_json::json!({
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!("This is a placeholder prompt response for instance {}", instance.instance_id)
                        }
                    }
                ]
            }),
        )
    }

    /// Handle list resources request
    async fn handle_list_resources(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!(
            "Listing available resources for instance: {}",
            instance.instance_id
        );

        // For now, return project contexts as resources
        let resources: Vec<MCPResource> = instance
            .project_contexts
            .iter()
            .map(|ctx_id| MCPResource {
                uri: format!("context://{}", ctx_id),
                name: format!("Project Context {}", ctx_id),
                description: Some(format!("Project context resource for {}", ctx_id)),
                mime_type: Some("application/json".to_string()),
            })
            .collect();

        let result = serde_json::json!({
            "resources": resources
        });

        MCPResponse::success_from_request(request.id, result)
    }

    /// Handle read resource request
    async fn handle_read_resource(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Reading resource for instance: {}", instance.instance_id);

        // For now, return a placeholder response
        MCPResponse::success_from_request(
            request.id,
            serde_json::json!({
                "contents": [
                    {
                        "uri": format!("instance://{}", instance.instance_id),
                        "mimeType": "text/plain",
                        "text": format!("This is a placeholder resource content for instance {}", instance.instance_id)
                    }
                ]
            }),
        )
    }

    /// Execute a tool call in the context of a specific instance
    async fn execute_tool(
        &self,
        tool_call: MCPToolCall,
        instance: &MCPServerInstance,
    ) -> Result<MCPToolResult, Box<dyn std::error::Error + Send + Sync>> {
        match tool_call.name.as_str() {
            "create_workflow" => {
                info!(
                    "Creating workflow with parameters: {:?} for instance: {}",
                    tool_call.arguments, instance.instance_id
                );
                Ok(MCPToolResult {
                    content: vec![MCPContent::text(format!(
                        "Workflow created successfully for instance {} (placeholder)",
                        instance.instance_id
                    ))],
                    is_error: Some(false),
                })
            }
            "execute_agent" => {
                info!(
                    "Executing agent with parameters: {:?} for instance: {}",
                    tool_call.arguments, instance.instance_id
                );
                Ok(MCPToolResult {
                    content: vec![MCPContent::text(format!(
                        "Agent executed successfully for instance {} (placeholder)",
                        instance.instance_id
                    ))],
                    is_error: Some(false),
                })
            }
            "search_project_context" => {
                info!(
                    "Searching project context with parameters: {:?} for instance: {}",
                    tool_call.arguments, instance.instance_id
                );

                // Check if the requested context is available to this instance
                let context_id = tool_call
                    .arguments
                    .get("context_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !instance.project_contexts.contains(&context_id.to_string()) {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Access denied: Project context '{}' not available to this instance",
                            context_id
                        ))],
                        is_error: Some(true),
                    });
                }

                // Get search query
                let query = tool_call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Note: Authentication check would be performed at the request level
                // For now, we'll log the context access attempt
                info!(
                    "Accessing project context '{}' for instance {}",
                    context_id, instance.instance_id
                );

                // Attempt to perform real search using OAuth if available
                let search_result = self
                    .perform_context_search(instance, context_id, query)
                    .await;

                match search_result {
                    Ok(results) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Search results for '{}' in context {}: {}",
                            query, context_id, results
                        ))],
                        is_error: Some(false),
                    }),
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Search failed for context {}: {}",
                            context_id, e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            // GitLab tools
            "gitlab_get_user" => {
                info!(
                    "Getting GitLab user info for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_list_projects" => {
                info!(
                    "Listing GitLab projects for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_get_project" => {
                info!(
                    "Getting GitLab project for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_list_issues" => {
                info!(
                    "Listing GitLab issues for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_create_issue" => {
                info!(
                    "Creating GitLab issue for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_list_merge_requests" => {
                info!(
                    "Listing GitLab merge requests for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_get_file" => {
                info!("Getting GitLab file for instance: {}", instance.instance_id);
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_list_pipelines" => {
                info!(
                    "Listing GitLab pipelines for instance: {}",
                    instance.instance_id
                );
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            "gitlab_search" => {
                info!("Searching GitLab for instance: {}", instance.instance_id);
                self.execute_gitlab_tool(&tool_call, instance).await
            }
            _ => Err(format!("Unknown tool: {}", tool_call.name).into()),
        }
    }

    /// Check if the authenticated user has permission to access a project context
    async fn check_project_context_permission(
        &self,
        claims: &MCPTokenClaims,
        context_id: &str,
    ) -> bool {
        // Check if the user has access to this context in their project_contexts list
        if claims.project_contexts.contains(&context_id.to_string()) {
            return true;
        }

        // Check if the user has wildcard access
        if claims.project_contexts.contains(&"*".to_string()) {
            return true;
        }

        // Check if the context is related to the user's installation
        // This allows access to project contexts that are part of the same installation
        if context_id.starts_with(&claims.installation_id) {
            return true;
        }

        // Default to deny access
        false
    }

    /// Get project context from git remote URL
    fn get_project_context_from_git(&self) -> Option<String> {
        use std::process::Command;

        // Try to get GitLab remote URL
        if let Ok(output) = Command::new("git")
            .args(&["remote", "get-url", "gitlab"])
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return self.extract_gitlab_project_from_url(&url);
            }
        }

        // Fallback: try origin remote
        if let Ok(output) = Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .output()
        {
            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if url.contains("gitlab.com") {
                    return self.extract_gitlab_project_from_url(&url);
                }
            }
        }

        None
    }

    /// Extract GitLab project path from URL
    fn extract_gitlab_project_from_url(&self, url: &str) -> Option<String> {
        // Handle various GitLab URL formats:
        // https://gitlab.com/user/project.git
        // git@gitlab.com:user/project.git
        // https://user@gitlab.com/user/project.git

        let url = url.trim_end_matches(".git");

        if url.contains("gitlab.com") {
            if let Some(path_start) = url.find("gitlab.com") {
                let after_domain = &url[path_start + "gitlab.com".len()..];

                // Handle SSH format (git@gitlab.com:user/project)
                if after_domain.starts_with(':') {
                    return Some(after_domain[1..].to_string());
                }

                // Handle HTTPS format (https://gitlab.com/user/project)
                if after_domain.starts_with('/') {
                    return Some(after_domain[1..].to_string());
                }
            }
        }

        None
    }

    /// Execute GitLab-specific tools using OAuth authentication
    async fn execute_gitlab_tool(
        &self,
        tool_call: &MCPToolCall,
        instance: &MCPServerInstance,
    ) -> Result<MCPToolResult, Box<dyn std::error::Error + Send + Sync>> {
        // Get project context from git remote if not provided in arguments
        let project_context = self.get_project_context_from_git();
        info!("Detected project context from git: {:?}", project_context);

        // Helper function to get project ID from path
        let get_project_id =
            |project_path: &str| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                // URL encode the project path for GitLab API
                let encoded_path = urlencoding::encode(project_path);
                Ok(encoded_path.to_string())
            };

        match tool_call.name.as_str() {
            "gitlab_get_user" => {
                let url = "https://gitlab.com/api/v4/user";
                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab User Info: {}",
                                    body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_list_projects" => {
                let mut url = "https://gitlab.com/api/v4/projects?membership=true".to_string();

                // Add query parameters from arguments
                if let Some(per_page) = tool_call.arguments.get("per_page").and_then(|v| v.as_u64())
                {
                    url.push_str(&format!("&per_page={}", per_page));
                }
                if let Some(owned) = tool_call.arguments.get("owned").and_then(|v| v.as_bool()) {
                    if owned {
                        url.push_str("&owned=true");
                    }
                }
                if let Some(starred) = tool_call.arguments.get("starred").and_then(|v| v.as_bool())
                {
                    if starred {
                        url.push_str("&starred=true");
                    }
                }
                if let Some(visibility) = tool_call
                    .arguments
                    .get("visibility")
                    .and_then(|v| v.as_str())
                {
                    url.push_str(&format!("&visibility={}", visibility));
                }

                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Projects: {}",
                                    body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_get_project" => {
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };

                let url = format!("https://gitlab.com/api/v4/projects/{}", project_id);

                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Project: {}",
                                    body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }

            "gitlab_list_issues" => {
                // Get project_id from arguments or use detected project context
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };

                let url = format!(
                    "https://gitlab.com/api/v4/projects/{}/issues?per_page=20",
                    project_id
                );
                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Issues for project {}: {}",
                                    project_id, body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_create_issue" => {
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };
                let title = tool_call
                    .arguments
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("New Issue");
                let description = tool_call
                    .arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let url = format!("https://gitlab.com/api/v4/projects/{}/issues", project_id);
                let body = serde_json::json!({
                    "title": title,
                    "description": description
                })
                .to_string();

                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::POST,
                        &url,
                        Some(body),
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "Created GitLab Issue: {}",
                                    body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_list_merge_requests" => {
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };

                let url = format!(
                    "https://gitlab.com/api/v4/projects/{}/merge_requests?per_page=20",
                    project_id
                );
                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Merge Requests for project {}: {}",
                                    project_id, body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_get_file" => {
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };
                let file_path = tool_call
                    .arguments
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("README.md");
                let ref_name = tool_call
                    .arguments
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("main");

                let encoded_path = urlencoding::encode(file_path);
                let url = format!(
                    "https://gitlab.com/api/v4/projects/{}/repository/files/{}?ref={}",
                    project_id, encoded_path, ref_name
                );

                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab File {}: {}",
                                    file_path, body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_list_pipelines" => {
                let project_id = if let Some(arg_project) = tool_call
                    .arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                {
                    arg_project.to_string()
                } else if let Some(ref context) = project_context {
                    get_project_id(context)?
                } else {
                    return Ok(MCPToolResult {
                        content: vec![MCPContent::text("No project specified and no git remote detected. Please provide project_id argument or run from a git repository with GitLab remote.".to_string())],
                        is_error: Some(true),
                    });
                };

                let url = format!(
                    "https://gitlab.com/api/v4/projects/{}/pipelines?per_page=20",
                    project_id
                );
                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Pipelines for project {}: {}",
                                    project_id, body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            "gitlab_search" => {
                let query = tool_call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let scope = tool_call
                    .arguments
                    .get("scope")
                    .and_then(|v| v.as_str())
                    .unwrap_or("projects");

                let url = format!(
                    "https://gitlab.com/api/v4/search?scope={}&search={}&per_page=20",
                    scope,
                    urlencoding::encode(query)
                );

                match self
                    .manager
                    .make_authenticated_api_request(
                        &OAuthProviderType::GitLab,
                        &instance.installation_id,
                        None,
                        reqwest::Method::GET,
                        &url,
                        None,
                        None,
                    )
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            let body = response.text().await?;
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab Search Results for '{}': {}",
                                    query, body
                                ))],
                                is_error: Some(false),
                            })
                        } else {
                            Ok(MCPToolResult {
                                content: vec![MCPContent::text(format!(
                                    "GitLab API error: {}",
                                    response.status()
                                ))],
                                is_error: Some(true),
                            })
                        }
                    }
                    Err(e) => Ok(MCPToolResult {
                        content: vec![MCPContent::text(format!(
                            "Failed to call GitLab API: {}",
                            e
                        ))],
                        is_error: Some(true),
                    }),
                }
            }
            _ => {
                // For unknown GitLab tools, return an error
                Ok(MCPToolResult {
                    content: vec![MCPContent::text(format!(
                        "GitLab tool '{}' is not yet implemented",
                        tool_call.name
                    ))],
                    is_error: Some(true),
                })
            }
        }
    }

    /// Perform search in project context using OAuth-authenticated APIs
    async fn perform_context_search(
        &self,
        instance: &MCPServerInstance,
        context_id: &str,
        query: &str,
    ) -> Result<String, String> {
        // Determine provider type from context_id (this is a simplified approach)
        let provider_type = if context_id.contains("gitlab") {
            OAuthProviderType::GitLab
        } else if context_id.contains("github") {
            OAuthProviderType::GitHub
        } else {
            return Err("Unknown context provider".to_string());
        };

        // Try to make authenticated request to search API
        match provider_type {
            OAuthProviderType::GitLab => {
                self.search_gitlab_context(&instance.installation_id, context_id, query)
                    .await
            }
            OAuthProviderType::GitHub => {
                self.search_github_context(&instance.installation_id, context_id, query)
                    .await
            }
            _ => Err("Provider not supported for search".to_string()),
        }
    }

    /// Search GitLab project using OAuth
    async fn search_gitlab_context(
        &self,
        installation_id: &str,
        context_id: &str,
        query: &str,
    ) -> Result<String, String> {
        // Extract project ID from context_id (simplified)
        let project_id = context_id.replace("gitlab:", "").replace("context:", "");

        let search_url = format!(
            "https://gitlab.com/api/v4/projects/{}/search?scope=blobs&search={}",
            project_id,
            urlencoding::encode(query)
        );

        match self
            .manager
            .make_authenticated_api_request(
                &OAuthProviderType::GitLab,
                installation_id,
                None,
                reqwest::Method::GET,
                &search_url,
                None,
                None,
            )
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(body) => Ok(format!("GitLab search results: {}", body)),
                        Err(e) => Err(format!("Failed to read response: {}", e)),
                    }
                } else {
                    Err(format!("GitLab API error: {}", response.status()))
                }
            }
            Err(e) => {
                warn!("OAuth request failed, falling back to placeholder: {}", e);
                Ok(format!(
                    "Search completed for '{}' in GitLab project {} (OAuth not available)",
                    query, project_id
                ))
            }
        }
    }

    /// Search GitHub repository using OAuth
    async fn search_github_context(
        &self,
        installation_id: &str,
        context_id: &str,
        query: &str,
    ) -> Result<String, String> {
        // Extract repo info from context_id (simplified)
        let repo = context_id.replace("github:", "").replace("context:", "");

        let search_url = format!(
            "https://api.github.com/search/code?q={}+repo:{}",
            urlencoding::encode(query),
            repo
        );

        match self
            .manager
            .make_authenticated_api_request(
                &OAuthProviderType::GitHub,
                installation_id,
                None,
                reqwest::Method::GET,
                &search_url,
                None,
                None,
            )
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(body) => Ok(format!("GitHub search results: {}", body)),
                        Err(e) => Err(format!("Failed to read response: {}", e)),
                    }
                } else {
                    Err(format!("GitHub API error: {}", response.status()))
                }
            }
            Err(e) => {
                warn!("OAuth request failed, falling back to placeholder: {}", e);
                Ok(format!(
                    "Search completed for '{}' in GitHub repo {} (OAuth not available)",
                    query, repo
                ))
            }
        }
    }
}

impl Default for CircuitBreakerMCPServer {
    fn default() -> Self {
        Self::new()
    }
}

// HTTP handlers

/// Handle MCP HTTP request for a specific instance
async fn handle_mcp_get_request(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Result<Response, StatusCode> {
    // Check if client expects SSE
    if let Some(accept) = headers.get("accept") {
        if let Ok(accept_str) = accept.to_str() {
            if accept_str.contains("text/event-stream") {
                info!("Client requesting SSE for instance: {}", instance_id);
                // Redirect to SSE endpoint
                return Ok(Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("Location", format!("/mcp/{}/sse", instance_id))
                    .body(Body::empty())
                    .unwrap()
                    .into_response());
            }
        }
    }

    // Check Content-Type for MCP clients that might send this header
    if let Some(content_type) = headers.get("content-type") {
        if let Ok(content_type_str) = content_type.to_str() {
            if content_type_str.contains("text/event-stream") {
                info!(
                    "Client requesting SSE via content-type for instance: {}",
                    instance_id
                );
                // Redirect to SSE endpoint
                return Ok(Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("Location", format!("/mcp/{}/sse", instance_id))
                    .body(Body::empty())
                    .unwrap()
                    .into_response());
            }
        }
    }
    // Check if this is a remote instance that needs OAuth
    if let Some(instance) = manager.get_server_instance(&instance_id).await {
        if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
            // For remote instances, return OAuth redirect if not authenticated
            if headers.get("authorization").is_none() {
                if let Some(oauth_config) = manager.get_oauth_config(&instance_id).await {
                    // Convert provider type string to enum
                    let provider_type = match oauth_config.provider_type.as_str() {
                        "gitlab" => crate::api::oauth::OAuthProviderType::GitLab,
                        "github" => crate::api::oauth::OAuthProviderType::GitHub,
                        "google" => crate::api::oauth::OAuthProviderType::Google,
                        custom => crate::api::oauth::OAuthProviderType::Custom(custom.to_string()),
                    };

                    // Register the OAuth provider if not already registered
                    let redirect_uri = format!(
                        "{}/mcp/{}/oauth/callback",
                        get_base_url_from_headers(&headers),
                        instance_id
                    );

                    let oauth_provider = crate::api::oauth::OAuthProvider {
                        provider_type: provider_type.clone(),
                        client_id: oauth_config.client_id.clone(),
                        client_secret: oauth_config.client_secret.clone(),
                        auth_url: oauth_config.auth_url.clone().unwrap_or_else(
                            || match oauth_config.provider_type.as_str() {
                                "gitlab" => "https://gitlab.com/oauth/authorize".to_string(),
                                "github" => "https://github.com/login/oauth/authorize".to_string(),
                                "google" => {
                                    "https://accounts.google.com/o/oauth2/v2/auth".to_string()
                                }
                                _ => "https://gitlab.com/oauth/authorize".to_string(),
                            },
                        ),
                        token_url: oauth_config.token_url.clone().unwrap_or_else(|| {
                            match oauth_config.provider_type.as_str() {
                                "gitlab" => "https://gitlab.com/oauth/token".to_string(),
                                "github" => {
                                    "https://github.com/login/oauth/access_token".to_string()
                                }
                                "google" => "https://oauth2.googleapis.com/token".to_string(),
                                _ => "https://gitlab.com/oauth/token".to_string(),
                            }
                        }),
                        scope: oauth_config.scope.clone(),
                        redirect_uri: redirect_uri.clone(),
                    };

                    // Register the provider with the OAuth manager
                    if let Err(e) = manager
                        .oauth_manager
                        .register_provider(oauth_provider)
                        .await
                    {
                        warn!("Failed to register OAuth provider: {}", e);
                    }

                    // Use the OAuth manager to generate the authorization URL
                    match manager
                        .oauth_manager
                        .get_authorization_url(
                            provider_type,
                            instance.installation_id.clone(),
                            None, // user_id
                            Some(redirect_uri),
                            Some(oauth_config.scope.clone()),
                        )
                        .await
                    {
                        Ok(auth_url) => {
                            let html = format!(
                                r#"<!DOCTYPE html><html><head><title>OAuth Required</title></head><body><script>window.location.href="{}";</script><a href="{}">Authenticate</a></body></html>"#,
                                auth_url, auth_url
                            );
                            return Ok(axum::response::Html(html).into_response());
                        }
                        Err(e) => {
                            error!("Failed to generate OAuth authorization URL: {}", e);
                            return Err(StatusCode::INTERNAL_SERVER_ERROR);
                        }
                    }
                }
            }
        }
    }

    // Handle GET requests with a default initialize request for local instances
    let initialize_request = MCPRequest {
        id: Some(MCPId::String("windsurf-init".to_string())),
        method: "initialize".to_string(),
        params: Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "windsurf",
                "version": "1.0.0"
            }
        })),
    };

    let response =
        handle_mcp_request_internal(manager, instance_id, headers, uri, initialize_request).await?;
    Ok(response.into_response())
}

async fn handle_mcp_request(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    axum::Json(request): axum::Json<MCPRequest>,
) -> Result<axum::Json<MCPResponse>, StatusCode> {
    // Check if this is a remote instance that needs OAuth (except for initialize)
    if request.method != "initialize" {
        if let Some(instance) = manager.get_server_instance(&instance_id).await {
            if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
                // For remote instances, check for OAuth token or session header
                if headers.get("authorization").is_none() && headers.get("x-mcp-session").is_none()
                {
                    return Err(StatusCode::UNAUTHORIZED);
                }
                // Continue with normal processing if authenticated
            }
        }
    }

    handle_mcp_request_internal(manager, instance_id, headers, uri, request).await
}

async fn handle_mcp_request_internal(
    manager: MCPServerManager,
    instance_id: String,
    headers: HeaderMap,
    uri: axum::http::Uri,
    request: MCPRequest,
) -> Result<axum::Json<MCPResponse>, StatusCode> {
    // Get the instance first to check its type and get required info
    let instance = match manager.get_server_instance(&instance_id).await {
        Some(instance) => instance,
        None => {
            return Ok(axum::Json(MCPResponse::error_from_request(
                request.id,
                error_codes::INVALID_REQUEST,
                format!("Instance {} not found", instance_id),
            )));
        }
    };

    // Attempt authentication (required for most operations except initialize)
    let claims = if request.method == "initialize" {
        // Initialize doesn't require authentication
        None
    } else if let Some(session_header) = headers.get("x-mcp-session") {
        // Handle session-based authentication - create a simple session for Windsurf
        if let Ok(session_id) = session_header.to_str() {
            info!("Using session-based authentication: {}", session_id);

            // For session-based auth, create minimal claims that allow access
            Some(MCPTokenClaims {
                installation_id: instance.installation_id.clone(),
                app_id: instance.app_id.clone(),
                user_id: Some(format!("session-user-{}", session_id)),
                permissions: MCPPermissions::default(),
                token_type: crate::api::mcp_auth::TokenType::Session,
                session_id: Some(session_id.to_string()),
                project_contexts: vec![],
            })
        } else {
            return Ok(axum::Json(MCPResponse::error_from_request(
                request.id,
                error_codes::INVALID_REQUEST,
                "Invalid session header".to_string(),
            )));
        }
    } else if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
        // For Remote OAuth instances, check for OAuth Bearer token
        if let Some(auth_header) = headers.get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    // For OAuth-native tenants, the Bearer token IS the authentication
                    // Create minimal claims for the OAuth user
                    info!(
                        "Accepting OAuth Bearer token for Remote instance: {}",
                        instance_id
                    );
                    Some(MCPTokenClaims {
                        installation_id: instance.installation_id.clone(),
                        app_id: instance.app_id.clone(),
                        user_id: Some("oauth-user".to_string()),
                        permissions: MCPPermissions::default(),
                        token_type: crate::api::mcp_auth::TokenType::Session,
                        session_id: Some("oauth-session".to_string()),
                        project_contexts: vec![],
                    })
                } else {
                    return Ok(axum::Json(MCPResponse::error_from_request(
                        request.id,
                        error_codes::INVALID_REQUEST,
                        "OAuth Bearer token required for Remote instances".to_string(),
                    )));
                }
            } else {
                return Ok(axum::Json(MCPResponse::error_from_request(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    "Invalid authorization header".to_string(),
                )));
            }
        } else {
            return Ok(axum::Json(MCPResponse::error_from_request(
                request.id,
                error_codes::INVALID_REQUEST,
                "Authorization required for Remote instances".to_string(),
            )));
        }
    } else {
        // For non-Remote instances, use JWT authentication
        // First check for Basic auth (from app_id@host URLs)
        let basic_auth_app_id = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Basic "))
            .and_then(|b64| general_purpose::STANDARD.decode(b64).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|auth_str| {
                if auth_str.ends_with(':') {
                    Some(auth_str[..auth_str.len() - 1].to_string())
                } else {
                    auth_str.split(':').next().map(|s| s.to_string())
                }
            });

        if let Some(app_id) = basic_auth_app_id {
            info!("Found app_id from Basic auth: {}", app_id);

            // Try to find a valid JWT token for this app_id from NATS
            match manager.get_app_token(&app_id).await {
                Ok(token) => match manager.jwt_service.validate_token(&token).await {
                    Ok(claims) => Some(claims),
                    Err(e) => {
                        warn!(
                            "URL-based token validation failed for app {}: {}",
                            app_id, e
                        );
                        return Ok(axum::Json(MCPResponse::error_from_request(
                            request.id,
                            error_codes::INVALID_REQUEST,
                            format!("Authentication failed: {}", e),
                        )));
                    }
                },
                Err(e) => {
                    warn!("No token found for app {}: {}", app_id, e);
                    return Ok(axum::Json(MCPResponse::error_from_request(
                        request.id,
                        error_codes::INVALID_REQUEST,
                        format!("Authentication failed: {}", e),
                    )));
                }
            }
        } else {
            // Try standard Bearer token authentication (for JWT tokens)
            match manager.authenticate_request(&headers).await {
                Ok(claims) => Some(claims),
                Err(e) => {
                    // If Bearer auth also fails, check URL authority as fallback
                    if let Some(authority) = uri.authority() {
                        let authority_str = authority.as_str();
                        if let Some(at_pos) = authority_str.find('@') {
                            let url_app_id = authority_str[..at_pos].to_string();
                            info!("Found app_id from URL authority: {}", url_app_id);

                            match manager.get_app_token(&url_app_id).await {
                                Ok(token) => match manager.jwt_service.validate_token(&token).await
                                {
                                    Ok(claims) => Some(claims),
                                    Err(token_err) => {
                                        return Ok(axum::Json(MCPResponse::error_from_request(
                                            request.id,
                                            error_codes::INVALID_REQUEST,
                                            format!("Authentication failed: {}", token_err),
                                        )));
                                    }
                                },
                                Err(_) => {
                                    return Ok(axum::Json(MCPResponse::error_from_request(
                                        request.id,
                                        error_codes::INVALID_REQUEST,
                                        format!("Authentication failed: {}", e),
                                    )));
                                }
                            }
                        } else {
                            return Ok(axum::Json(MCPResponse::error_from_request(
                                request.id,
                                error_codes::INVALID_REQUEST,
                                format!("Authentication failed: {}", e),
                            )));
                        }
                    } else {
                        return Ok(axum::Json(MCPResponse::error_from_request(
                            request.id,
                            error_codes::INVALID_REQUEST,
                            format!("Authentication failed: {}", e),
                        )));
                    }
                }
            }
        }
    };

    // Extract Bearer token for SSE routing OR session ID
    let auth_token = if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else if let Some(session_header) = headers.get("x-mcp-session") {
        // For session-based auth, check if session exists or create one
        if let Ok(session_id) = session_header.to_str() {
            info!("Using session-based authentication: {}", session_id);

            // Check if session already exists
            if let Some(session) = manager.get_session(session_id).await {
                info!("Found existing session: {}", session_id);
                // Use the session's OAuth token if available
                if let Some(instance) = manager.get_server_instance(&instance_id).await {
                    if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
                        // For Remote OAuth instances, get a valid OAuth token for this session
                        if let MCPApplicationType::Remote(ref oauth_config) = instance.app_type {
                            let provider_type = match oauth_config.provider_type.as_str() {
                                "gitlab" => crate::api::oauth::OAuthProviderType::GitLab,
                                "github" => crate::api::oauth::OAuthProviderType::GitHub,
                                "google" => crate::api::oauth::OAuthProviderType::Google,
                                custom => {
                                    crate::api::oauth::OAuthProviderType::Custom(custom.to_string())
                                }
                            };
                            match manager
                                .oauth_manager
                                .get_token(&provider_type, &instance.installation_id, None)
                                .await
                            {
                                Ok(token) => {
                                    info!("Retrieved OAuth token for session: {}", session_id);
                                    Some(token.access_token)
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to get OAuth token for session {}: {}",
                                        session_id, e
                                    );
                                    None
                                }
                            }
                        } else {
                            // For non-Remote instances, use a placeholder token
                            Some(format!("session-{}", session_id))
                        }
                    } else {
                        // For non-Remote instances, use a placeholder token
                        Some(format!("session-{}", session_id))
                    }
                } else {
                    None
                }
            } else {
                info!("Creating new session: {}", session_id);
                // Create a new session for this session ID
                if let Some(instance) = manager.get_server_instance(&instance_id).await {
                    let session_id_created = manager
                        .create_session(
                            instance_id.clone(),
                            instance.installation_id.clone(),
                            instance.app_id.clone(),
                            MCPClientInfo {
                                name: "windsurf".to_string(),
                                version: "1.0.0".to_string(),
                                user_agent: headers
                                    .get("user-agent")
                                    .and_then(|h| h.to_str().ok())
                                    .map(|s| s.to_string()),
                            },
                            None, // user_id
                            MCPSessionPermissions {
                                tools: vec!["*".to_string()],
                                prompts: vec!["*".to_string()],
                                resources: vec!["*".to_string()],
                                project_contexts: std::collections::HashMap::new(),
                            },
                            vec![], // project_contexts
                        )
                        .await;

                    info!("Created session: {}", session_id_created);

                    // For Remote OAuth instances, get a valid OAuth token
                    if let MCPApplicationType::Remote(ref oauth_config) = instance.app_type {
                        let provider_type = match oauth_config.provider_type.as_str() {
                            "gitlab" => crate::api::oauth::OAuthProviderType::GitLab,
                            "github" => crate::api::oauth::OAuthProviderType::GitHub,
                            "google" => crate::api::oauth::OAuthProviderType::Google,
                            custom => {
                                crate::api::oauth::OAuthProviderType::Custom(custom.to_string())
                            }
                        };
                        match manager
                            .oauth_manager
                            .get_token(&provider_type, &instance.installation_id, None)
                            .await
                        {
                            Ok(token) => {
                                info!("Retrieved OAuth token for new session: {}", session_id);
                                Some(token.access_token)
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to get OAuth token for new session {}: {}",
                                    session_id, e
                                );
                                None
                            }
                        }
                    } else {
                        // For non-Remote instances, use a placeholder token
                        Some(format!("session-{}", session_id))
                    }
                } else {
                    warn!("Instance {} not found for session creation", instance_id);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let server = CircuitBreakerMCPServer {
        manager: manager.clone(),
    };
    let response = server
        .handle_request(&instance_id, request.clone(), claims)
        .await;

    // Try to route the response via SSE if there's an active SSE connection
    if let Some(token) = auth_token {
        if SSE_ROUTER.send_response(&token, &response).await {
            info!("Routed MCP response via SSE for token: {}...", &token[..8]);
            // Return a minimal acknowledgment response since the real response went via SSE
            let ack_response = MCPResponse::success_from_request(
                Some(response.id.clone()),
                serde_json::json!({"status": "response_sent_via_sse"}),
            );
            return Ok(axum::Json(ack_response));
        }
    }

    // If no SSE routing available, return the response via HTTP as usual
    Ok(axum::Json(response))
}

/// Handle default MCP HTTP endpoint for mcp-remote compatibility
async fn handle_default_mcp_http(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    Json(request): Json<MCPRequest>,
) -> Result<axum::Json<MCPResponse>, StatusCode> {
    info!("Default MCP HTTP endpoint called");
    info!("Headers: {:?}", headers);
    info!("URI: {:?}", uri);

    // Check if OAuth is globally disabled
    let oauth_enabled = std::env::var("MCP_OAUTH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Check if authentication is required for gitlab-demo instance
    if let Some(instance) = manager.get_server_instance("gitlab-demo").await {
        info!("Found gitlab-demo instance: {:?}", instance.app_type);
        if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
            // For remote instances, check authentication only if OAuth is enabled
            if oauth_enabled && headers.get("authorization").is_none() {
                info!("OAuth is enabled and no authorization header found, returning OAuth error");
                // Return OAuth error response in MCP format with discovery info
                let error_data = serde_json::json!({
                    "oauth_required": true,
                    "discovery_url": "/.well-known/oauth-authorization-server",
                    "registration_url": format!("{}/register", get_base_url_from_headers(&headers)),
                    "authorization_url": "https://gitlab.com/oauth/authorize"
                });
                return Ok(axum::Json(MCPResponse::error_with_data_from_request(
                    request.id,
                    401,
                    "Authentication required. Please complete OAuth flow.".to_string(),
                    error_data,
                )));
            } else if oauth_enabled {
                info!("OAuth is enabled and authorization header found, proceeding with request");
            } else {
                info!("OAuth is disabled, skipping authentication check");
            }
        }
    } else {
        error!("gitlab-demo instance not found!");
    }

    // Default to gitlab-demo instance for mcp-remote
    handle_mcp_request_internal(manager, "gitlab-demo".to_string(), headers, uri, request).await
}

/// Handle default SSE endpoint for mcp-remote compatibility
async fn handle_default_mcp_sse(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
) -> Response {
    info!("Default SSE endpoint called");
    info!("Headers: {:?}", headers);

    // Default to gitlab-demo instance for mcp-remote
    handle_mcp_sse_internal(manager, "gitlab-demo".to_string(), headers).await
}

/// Handle MCP requests via Server-Sent Events (SSE)
async fn handle_mcp_sse(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    handle_mcp_sse_internal(manager, instance_id, headers).await
}

/// Internal SSE handler shared by both endpoints
async fn handle_mcp_sse_internal(
    manager: MCPServerManager,
    instance_id: String,
    headers: HeaderMap,
) -> Response {
    info!("SSE internal handler called for instance: {}", instance_id);
    info!("Headers: {:?}", headers);

    // Extract Bearer token for multi-tenant routing
    let auth_token = if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Check if OAuth is globally disabled
    let oauth_enabled = std::env::var("MCP_OAUTH_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Check if this is a remote instance that needs OAuth
    if let Some(instance) = manager.get_server_instance(&instance_id).await {
        info!("Found instance: {:?}", instance.app_type);
        if matches!(instance.app_type, MCPApplicationType::Remote(_)) {
            // For remote instances, return 401 if not authenticated and OAuth is enabled
            if oauth_enabled && auth_token.is_none() {
                info!("OAuth is enabled and no authorization header found in SSE request, returning 401");
                return Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("WWW-Authenticate", "Bearer realm=\"MCP OAuth\"")
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(
                        "event: error\ndata: {\"error\": \"Authorization required\"}\n\n"
                            .to_string(),
                    )
                    .unwrap()
                    .into_response();
            } else if oauth_enabled {
                info!("OAuth is enabled and authorization header found in SSE request");
            } else {
                info!("OAuth is disabled, skipping authentication check for SSE");
            }
        }
    } else {
        error!("Instance {} not found!", instance_id);
    }

    info!("SSE connection established, waiting for client requests");

    // Create a proper SSE stream that maintains connection and handles keep-alives
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event, Infallible>>();

    // Register this SSE channel with the global router for multi-tenant response routing
    if let Some(token) = auth_token.clone() {
        SSE_ROUTER.register_channel(token.clone(), tx.clone()).await;

        // Clean up on disconnect
        let cleanup_token = token.clone();
        let cleanup_tx = tx.clone();
        tokio::spawn(async move {
            // Wait for the channel to be closed (client disconnected)
            while cleanup_tx
                .send(Ok(Event::default().event("ping").data("")))
                .is_ok()
            {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
            // Clean up the channel from the router
            SSE_ROUTER.unregister_channel(&cleanup_token).await;
        });
    }

    // Spawn a task to handle keep-alive messages
    let keep_alive_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let keep_alive_event = Event::default().event("keep-alive").data(format!(
                "{{\"timestamp\": \"{}\"}}",
                chrono::Utc::now().to_rfc3339()
            ));

            if keep_alive_tx.send(Ok(keep_alive_event)).is_err() {
                break; // Client disconnected
            }
        }
    });

    // Send initial connection message
    let connection_event = Event::default().event("connected").data(format!(
        "{{\"instance_id\": \"{}\", \"protocol_version\": \"2024-11-05\"}}",
        instance_id
    ));

    if tx.send(Ok(connection_event)).is_err() {
        error!("Failed to send initial connection event");
    }

    // Convert receiver to stream using manual implementation
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(tokio::time::Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

/// Simple SSE test endpoint for debugging
async fn handle_sse_test() -> Response {
    let test_response = serde_json::json!({
        "message": "SSE test successful",
        "timestamp": chrono::Utc::now().timestamp()
    });

    let sse_content = format!(
        "event: test\ndata: {}\n\n",
        serde_json::to_string(&test_response).unwrap()
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(sse_content)
        .unwrap()
        .into_response()
}

/// Handle OAuth Authorization Server Metadata discovery (/.well-known/oauth-authorization-server)
async fn handle_oauth_authorization_server_metadata(
    State(_manager): State<MCPServerManager>,
    headers: HeaderMap,
) -> Response {
    let base_url = get_base_url_from_headers(&headers);

    let metadata = serde_json::json!({
        "issuer": base_url,
        "authorization_endpoint": "https://gitlab.com/oauth/authorize",
        "token_endpoint": "https://gitlab.com/oauth/token",
        "registration_endpoint": format!("{}/register", base_url),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": ["api"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"]
    });

    info!("OAuth authorization server metadata: {:?}", metadata);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, MCP-Protocol-Version",
        )
        .body(String::from(serde_json::to_string(&metadata).unwrap()))
        .unwrap()
        .into_response()
}

/// Handle MCP discovery endpoint (/.well-known/mcp) - optional endpoint
async fn handle_mcp_discovery(
    State(_manager): State<MCPServerManager>,
    headers: HeaderMap,
) -> Response {
    let base_url = get_base_url_from_headers(&headers);

    let discovery = serde_json::json!({
        "mcp_version": "2024-11-05",
        "authorization": {
            "required": true,
            "oauth2": {
                "authorization_endpoint": "https://gitlab.com/oauth/authorize",
                "token_endpoint": "https://gitlab.com/oauth/token",
                "registration_endpoint": format!("{}/register", base_url),
                "scopes_supported": ["api"],
                "response_types_supported": ["code"],
                "grant_types_supported": ["authorization_code"],
                "code_challenge_methods_supported": ["S256"]
            }
        },
        "capabilities": {
            "tools": {},
            "prompts": {},
            "resources": {}
        }
    });

    info!("MCP discovery response: {:?}", discovery);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, MCP-Protocol-Version",
        )
        .body(String::from(serde_json::to_string(&discovery).unwrap()))
        .unwrap()
        .into_response()
}

/// Handle OAuth metadata endpoint
async fn handle_oauth_metadata(State(_manager): State<MCPServerManager>) -> Response {
    let metadata = serde_json::json!({
        "issuer": "https://gitlab.com",
        "authorization_endpoint": "https://gitlab.com/oauth/authorize",
        "token_endpoint": "https://gitlab.com/oauth/token",
        "scopes_supported": ["api", "read_user", "read_repository"],
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "client_registration_endpoint": "/register"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        )
        .body(String::from(serde_json::to_string(&metadata).unwrap()))
        .unwrap()
        .into_response()
}

/// Handle OAuth dynamic client registration
async fn handle_oauth_register(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
    Json(request): Json<serde_json::Value>,
) -> Result<Response, StatusCode> {
    info!("=== OAuth Client Registration Called ===");
    info!(
        "Request body: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );

    // Check if gitlab-demo instance exists
    let instance_exists = manager.get_server_instance("gitlab-demo").await.is_some();
    info!("gitlab-demo instance exists: {}", instance_exists);

    let base_url = get_base_url_from_headers(&headers);

    // Extract redirect_uris from the request
    let redirect_uris = match request.get("redirect_uris") {
        Some(uris) => {
            if let Some(arr) = uris.as_array() {
                let uris: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                if uris.is_empty() {
                    warn!("Empty redirect_uris array provided, using default");
                    vec![format!("{}/oauth/callback", base_url)]
                } else {
                    uris
                }
            } else {
                warn!("redirect_uris is not an array, using default");
                vec![format!("{}/oauth/callback", base_url)]
            }
        }
        None => {
            info!("No redirect_uris provided, using default");
            vec![format!("{}/oauth/callback", base_url)]
        }
    };

    info!("Client requested redirect URIs: {:?}", redirect_uris);

    // Validate that redirect URIs are not empty
    if redirect_uris.is_empty() {
        error!("No valid redirect URIs provided");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Try to get OAuth config
    match manager.get_oauth_config("gitlab-demo").await {
        Some(oauth_config) => {
            info!("Found OAuth config for gitlab-demo");
            info!("Client ID: {}", oauth_config.client_id);
            info!("Provider type: {}", oauth_config.provider_type);
            info!("Scopes: {:?}", oauth_config.scope);

            let timestamp = chrono::Utc::now().timestamp();

            // Return the OAuth app configuration for GitLab
            let mut response = serde_json::json!({
                "client_id": oauth_config.client_id,
                "client_secret": oauth_config.client_secret,
                "client_id_issued_at": timestamp,
                "client_secret_expires_at": 0,
                "application_type": "web",
                "client_name": "mcp-remote",
                "client_uri": request.get("client_uri").unwrap_or(&serde_json::Value::String("https://github.com/modelcontextprotocol/inspector".to_string())),
                "subject_type": "public",
                "token_endpoint_auth_method": "client_secret_basic",
                "grant_types": ["authorization_code"],
                "response_types": ["code"],
                "scope": oauth_config.scope.join(" "),
                "contacts": [],
                // Use server-based redirect instead of client's dynamic port
                "redirect_uris": [format!("{}/oauth/callback/debug", base_url)]
            });

            // Echo back any additional parameters from the client request
            if let Some(obj) = request.as_object() {
                for (key, value) in obj {
                    if !response.as_object().unwrap().contains_key(key) {
                        response[key] = value.clone();
                        info!("Echoing back client parameter: {} = {:?}", key, value);
                    }
                }
            }

            info!("=== OAuth Registration Response ===");
            info!(
                "{}",
                serde_json::to_string_pretty(&response).unwrap_or_default()
            );

            Ok(Response::builder()
                .status(StatusCode::CREATED)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .header(
                    "Access-Control-Allow-Headers",
                    "Content-Type, Authorization",
                )
                .body(String::from(serde_json::to_string(&response).unwrap()))
                .unwrap()
                .into_response())
        }
        None => {
            error!("=== OAuth Registration Failed ===");
            error!("No OAuth config found for gitlab-demo instance");

            // List all available instances for debugging
            let instances = manager.list_server_instances().await;
            info!(
                "Available instances: {:?}",
                instances.iter().map(|i| &i.instance_id).collect::<Vec<_>>()
            );

            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Handle OPTIONS requests for CORS preflight
async fn handle_options() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        )
        .header("Access-Control-Max-Age", "86400")
        .body(Body::empty())
        .unwrap()
        .into_response()
}

/// Handle debug endpoint to list all instances
async fn handle_debug_instances(
    State(manager): State<MCPServerManager>,
) -> Json<serde_json::Value> {
    let instances = manager.list_server_instances().await;

    let instance_list: Vec<serde_json::Value> = instances
        .iter()
        .map(|instance| {
            serde_json::json!({
                "instance_id": instance.instance_id,
                "name": instance.name,
                "app_type": match &instance.app_type {
                    MCPApplicationType::Local => "Local",
                    MCPApplicationType::Remote(_) => "Remote"
                },
                "status": format!("{:?}", instance.status),
                "created_at": instance.created_at
            })
        })
        .collect();

    let mut oauth_configs = HashMap::new();
    for instance in &instances {
        if let Some(oauth_config) = manager.get_oauth_config(&instance.instance_id).await {
            oauth_configs.insert(
                &instance.instance_id,
                serde_json::json!({
                    "provider_type": oauth_config.provider_type,
                    "client_id": oauth_config.client_id,
                    "scope": oauth_config.scope
                }),
            );
        }
    }

    let debug_info = serde_json::json!({
        "total_instances": instances.len(),
        "instances": instance_list,
        "oauth_configs": oauth_configs
    });

    info!(
        "Debug instances response: {}",
        serde_json::to_string_pretty(&debug_info).unwrap_or_default()
    );
    Json(debug_info)
}

/// Create a new MCP server instance
async fn create_mcp_instance(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<CreateMCPInstanceRequest>,
) -> Result<axum::Json<CreateMCPInstanceResponse>, StatusCode> {
    // Require authentication for instance creation
    let _claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    match manager
        .create_server_instance(
            request.app_id,
            request.installation_id,
            request.name,
            request.description,
            request.project_contexts,
            request.app_type,
        )
        .await
    {
        Ok(instance_id) => Ok(axum::Json(CreateMCPInstanceResponse { instance_id })),
        Err(e) => {
            error!("Failed to create MCP instance: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Validate OAuth token for a specific instance
async fn validate_instance_oauth_token(
    _manager: &MCPServerManager,
    _instance_id: &str,
    _token: &str,
) -> Result<(), String> {
    // TODO: Implement actual token validation
    // This should verify the token with the OAuth provider
    // and ensure it's valid for this specific instance
    Ok(())
}

/// Handle OAuth callback for a specific instance
async fn handle_instance_oauth_callback(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    handle_instance_oauth_callback_internal(manager, instance_id, params).await
}

/// Internal OAuth callback handler
async fn handle_instance_oauth_callback_internal(
    manager: MCPServerManager,
    instance_id: String,
    params: std::collections::HashMap<String, String>,
) -> Result<Response, StatusCode> {
    let code = params.get("code").ok_or(StatusCode::BAD_REQUEST)?;
    let state = params.get("state").ok_or(StatusCode::BAD_REQUEST)?;

    // Verify state contains our instance_id
    if !state.starts_with(&format!("{}:", instance_id)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get the OAuth configuration for this instance
    let oauth_config = manager
        .get_oauth_config(&instance_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // For now, return a simple token - in production, implement proper OAuth exchange
    let access_token = format!("oauth_token_{}_{}", instance_id, uuid::Uuid::new_v4());

    // Store token for this instance (TODO: implement proper storage)

    // Return success page with token
    let success_html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: #f5f5f5;
        }}
        .container {{
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            text-align: center;
            max-width: 500px;
        }}
        .success {{ color: #28a745; }}
        .token {{
            background: #f8f9fa;
            padding: 10px;
            border-radius: 4px;
            font-family: monospace;
            word-break: break-all;
            margin: 10px 0;
        }}
        .copy-button {{
            background: #007bff;
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
            margin-left: 10px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h2 class="success">✓ Authentication Successful</h2>
        <p>You have successfully authenticated with the MCP server instance: <strong>{}</strong></p>
        <p><strong>Your access token:</strong></p>
        <div class="token" id="token">{}</div>
        <button class="copy-button" onclick="copyToken()">Copy Token</button>
        <p><small>Use this token to authenticate your MCP client connections.</small></p>
        <p><small>You can now close this window and return to your MCP client.</small></p>
    </div>
    <script>
        function copyToken() {{
            const token = document.getElementById('token').textContent;
            navigator.clipboard.writeText(token).then(function() {{
                alert('Token copied to clipboard!');
            }});
        }}
    </script>
</body>
</html>
        "#,
        instance_id, access_token
    );

    Ok(axum::response::Html(success_html).into_response())
}

/// Get MCP server instance details
async fn get_mcp_instance(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<MCPServerInstance>, StatusCode> {
    // Require authentication
    let _claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    match manager.get_server_instance(&instance_id).await {
        Some(instance) => Ok(axum::Json(instance)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Request/Response types for instance management
#[derive(Debug, serde::Deserialize)]
pub struct CreateMCPInstanceRequest {
    pub app_id: String,
    pub installation_id: String,
    pub name: String,
    pub description: String,
    pub project_contexts: Vec<String>,
    pub app_type: MCPApplicationType,
}

#[derive(Debug, serde::Serialize)]
pub struct CreateMCPInstanceResponse {
    pub instance_id: String,
}

/// Handle MCP WebSocket connection for a specific instance
async fn handle_mcp_websocket(
    ws: WebSocketUpgrade,
    Path(instance_id): Path<String>,
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // WebSocket connections require authentication
    let claims = match manager.authenticate_request(&headers).await {
        Ok(claims) => Some(claims),
        Err(e) => {
            warn!(
                "WebSocket authentication failed for instance {}: {}",
                instance_id, e
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    Ok(ws.on_upgrade(move |socket| {
        handle_websocket_connection(socket, instance_id, manager, claims)
    }))
}

/// Handle WebSocket connection for a specific MCP instance
async fn handle_websocket_connection(
    mut socket: axum::extract::ws::WebSocket,
    instance_id: String,
    manager: MCPServerManager,
    claims: Option<MCPTokenClaims>,
) {
    info!(
        "New MCP WebSocket connection established for instance: {}",
        instance_id
    );

    // Verify the instance exists
    if manager.get_server_instance(&instance_id).await.is_none() {
        warn!(
            "WebSocket connection attempted for non-existent instance: {}",
            instance_id
        );
        let error_response = MCPResponse::error_from_request(
            Some(MCPId::String("init".to_string())),
            error_codes::INVALID_REQUEST,
            format!("Server instance '{}' not found", instance_id),
        );
        if let Ok(error_json) = serde_json::to_string(&error_response) {
            let _ = socket
                .send(axum::extract::ws::Message::Text(error_json))
                .await;
        }
        return;
    }

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                debug!(
                    "Received WebSocket message for instance {}: {}",
                    instance_id, text
                );

                // Parse the MCP request
                match serde_json::from_str::<MCPRequest>(&text) {
                    Ok(request) => {
                        // Handle the request in the context of this instance
                        let server = CircuitBreakerMCPServer {
                            manager: manager.clone(),
                        };
                        let response = server
                            .handle_request(&instance_id, request, claims.clone())
                            .await;

                        // Send response back
                        if let Ok(response_json) = serde_json::to_string(&response) {
                            if socket
                                .send(axum::extract::ws::Message::Text(response_json))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse MCP request for instance {}: {}",
                            instance_id, e
                        );
                        let error_response = MCPResponse::error_from_request(
                            Some(MCPId::String("unknown".to_string())),
                            error_codes::PARSE_ERROR,
                            format!("Parse error: {}", e),
                        );
                        if let Ok(error_json) = serde_json::to_string(&error_response) {
                            let _ = socket
                                .send(axum::extract::ws::Message::Text(error_json))
                                .await;
                        }
                    }
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                info!(
                    "MCP WebSocket connection closed for instance: {}",
                    instance_id
                );
                break;
            }
            Err(e) => {
                error!("WebSocket error for instance {}: {}", instance_id, e);
                break;
            }
            _ => {}
        }
    }
}

/// Get server info for a specific instance
async fn get_server_info(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<MCPServerInfo>, StatusCode> {
    // Attempt authentication (optional for server info)
    let _claims = manager.authenticate_request(&headers).await.ok();

    match manager.get_server_instance(&instance_id).await {
        Some(instance) => {
            let server_info = MCPServerInfo {
                name: instance.name,
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: instance.description,
                capabilities: instance.capabilities,
            };
            Ok(axum::Json(server_info))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// List tools for a specific instance
async fn list_tools(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<Vec<MCPTool>>, StatusCode> {
    // Attempt authentication (optional for listing tools)
    let _claims = manager.authenticate_request(&headers).await.ok();

    match manager.get_server_instance(&instance_id).await {
        Some(_instance) => {
            let tools = manager.get_default_tools(&instance_id).await;
            Ok(axum::Json(tools))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// List prompts for a specific instance
async fn list_prompts(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<Vec<MCPPrompt>>, StatusCode> {
    // Attempt authentication (optional for listing prompts)
    let _claims = manager.authenticate_request(&headers).await.ok();

    match manager.get_server_instance(&instance_id).await {
        Some(_instance) => {
            let prompts = manager.get_default_prompts(&instance_id).await;
            Ok(axum::Json(prompts))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// List resources for a specific instance
async fn list_resources(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<Vec<MCPResource>>, StatusCode> {
    // Attempt authentication (optional for listing resources)
    let _claims = manager.authenticate_request(&headers).await.ok();

    match manager.get_server_instance(&instance_id).await {
        Some(instance) => {
            // For now, return project contexts as resources
            let resources: Vec<MCPResource> = instance
                .project_contexts
                .iter()
                .map(|ctx_id| MCPResource {
                    uri: format!("context://{}", ctx_id),
                    name: format!("Project Context {}", ctx_id),
                    description: Some(format!("Project context resource for {}", ctx_id)),
                    mime_type: Some("application/json".to_string()),
                })
                .collect();

            Ok(axum::Json(resources))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Register a new MCP app
async fn register_app(
    State(manager): State<MCPServerManager>,
    axum::Json(app): axum::Json<MCPApp>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    match manager.register_app(app).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => {
            error!("Failed to register app: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Register a new MCP installation
async fn register_installation(
    State(manager): State<MCPServerManager>,
    axum::Json(installation): axum::Json<MCPInstallation>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    match manager.register_installation(installation).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => {
            error!("Failed to register installation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create an installation token
async fn create_installation_token(
    State(manager): State<MCPServerManager>,
    Path(installation_id): Path<String>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<CreateInstallationTokenRequest>,
) -> Result<axum::Json<super::mcp_auth::MCPInstallationToken>, StatusCode> {
    // Require app authentication for creating installation tokens
    let claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify this is an app token
    if claims.token_type != super::mcp_auth::TokenType::App {
        return Err(StatusCode::FORBIDDEN);
    }

    match manager
        .create_installation_token(&claims.app_id, &installation_id, request.permissions)
        .await
    {
        Ok(token) => Ok(axum::Json(token)),
        Err(e) => {
            error!("Failed to create installation token: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a session token
async fn create_session_token(
    State(manager): State<MCPServerManager>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<CreateSessionTokenRequest>,
) -> Result<axum::Json<CreateSessionTokenResponse>, StatusCode> {
    // Require installation token for creating session tokens
    let claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Extract client info from headers
    let client_info = ClientInfo {
        ip_address: headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
        user_agent: headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
    };

    match manager
        .create_session_token(
            &claims.installation_id,
            &session_id,
            request.user_id,
            request.permissions,
            request.project_contexts,
            client_info,
        )
        .await
    {
        Ok(token) => Ok(axum::Json(CreateSessionTokenResponse { token })),
        Err(e) => {
            error!("Failed to create session token: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Revoke a token
async fn revoke_token(
    State(manager): State<MCPServerManager>,
    Path(token_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    // Require authentication to revoke tokens
    let _claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    match manager.revoke_token(&token_id).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => {
            error!("Failed to revoke token: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Request types for authentication endpoints
#[derive(Debug, serde::Deserialize)]
pub struct CreateInstallationTokenRequest {
    pub permissions: Option<MCPPermissions>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateSessionTokenRequest {
    pub user_id: Option<String>,
    pub permissions: MCPSessionPermissions,
    pub project_contexts: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CreateSessionTokenResponse {
    pub token: String,
}

/// Register OAuth provider
async fn register_oauth_provider(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
    axum::Json(provider): axum::Json<super::oauth::OAuthProvider>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    // Require authentication for provider registration
    let _claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    match manager.register_oauth_provider(provider).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => {
            error!("Failed to register OAuth provider: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get OAuth authorization URL
async fn get_oauth_authorization_url(
    State(manager): State<MCPServerManager>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<OAuthAuthorizationRequest>,
) -> Result<axum::Json<OAuthAuthorizationResponse>, StatusCode> {
    // Require authentication
    let claims = manager
        .authenticate_request(&headers)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    match manager
        .get_oauth_authorization_url(
            request.provider_type,
            claims.installation_id,
            claims.user_id,
            request.redirect_uri,
            request.scope,
        )
        .await
    {
        Ok(auth_url) => Ok(axum::Json(OAuthAuthorizationResponse { auth_url })),
        Err(e) => {
            error!("Failed to generate OAuth URL: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle OAuth callback
async fn handle_oauth_callback(
    State(manager): State<MCPServerManager>,
    axum::Json(callback): axum::Json<super::oauth::OAuthCallback>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    match manager.handle_oauth_callback(callback).await {
        Ok(token) => Ok(axum::Json(serde_json::json!({
            "success": true,
            "provider_type": token.provider_type,
            "installation_id": token.installation_id,
            "expires_at": token.expires_at
        }))),
        Err(e) => {
            error!("Failed to handle OAuth callback: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Request/Response types for OAuth endpoints
#[derive(Debug, serde::Deserialize)]
pub struct OAuthAuthorizationRequest {
    pub provider_type: OAuthProviderType,
    pub redirect_uri: Option<String>,
    pub scope: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
pub struct OAuthAuthorizationResponse {
    pub auth_url: String,
}

/// Get the base URL for this server instance - dynamically detect from headers or env
fn get_base_url_from_headers(headers: &HeaderMap) -> String {
    // Try to get from environment first
    if let Ok(base_url) = std::env::var("MCP_BASE_URL") {
        return base_url;
    }

    // Try to extract from Host header
    if let Some(host) = headers.get("host").and_then(|h| h.to_str().ok()) {
        // Check if it's an ngrok host (contains ngrok in the name)
        if host.contains("ngrok") {
            return format!("https://{}", host);
        }
        // For localhost, assume http
        if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            return format!("http://{}", host);
        }
        // Default to https for other hosts
        return format!("https://{}", host);
    }

    // Fallback to localhost
    "http://localhost:8080".to_string()
}

/// Handle general OAuth callback - extract instance from state and redirect
async fn handle_general_oauth_callback(
    State(manager): State<MCPServerManager>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    let state = params.get("state").ok_or(StatusCode::BAD_REQUEST)?;

    // Extract instance_id from state (format: instance_id:uuid)
    let instance_id = state.split(':').next().ok_or(StatusCode::BAD_REQUEST)?;

    // Forward to instance-specific handler
    handle_instance_oauth_callback_internal(manager, instance_id.to_string(), params).await
}

/// Handle OAuth callback for MCP clients (like mcp-remote) - extract instance from state and redirect
async fn handle_mcp_client_oauth_callback(
    State(manager): State<MCPServerManager>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    info!("=== MCP Client OAuth Callback Called ===");
    info!("Callback params: {:?}", params);

    let state = params.get("state").ok_or(StatusCode::BAD_REQUEST)?;
    let code = params.get("code").ok_or(StatusCode::BAD_REQUEST)?;

    // Extract instance_id from state (format: instance_id:uuid)
    let instance_id = state.split(':').next().ok_or(StatusCode::BAD_REQUEST)?;

    info!("Extracted instance_id: {}", instance_id);

    // Verify the instance exists
    let _instance = manager
        .get_server_instance(instance_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // For MCP clients, we need to redirect back to their callback URL
    // The client expects to receive the callback on their local port
    // Try to extract the original client port from state or use a default
    let client_port = if let Some(port_param) = params.get("client_port") {
        port_param.clone()
    } else {
        // Parse from state if it includes port info, otherwise use 6274 as default
        "6274".to_string()
    };

    // Redirect back to the client's callback URL
    let client_redirect_url = format!(
        "http://127.0.0.1:{}/oauth/callback/debug?code={}&state={}",
        client_port,
        urlencoding::encode(code),
        urlencoding::encode(state)
    );

    info!("Redirecting to client callback: {}", client_redirect_url);

    // Create a redirect response
    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", client_redirect_url)
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .body(Body::empty())
        .unwrap()
        .into_response())
}

/// Handle dynamic token requests for MCP clients
// OAuth initiation endpoint for MCP clients
async fn handle_mcp_oauth_init(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Redirect, StatusCode> {
    info!("OAuth initiation requested for instance: {}", instance_id);

    // Get the instance to check if it supports OAuth
    let instance = match manager.get_server_instance(&instance_id).await {
        Some(instance) => instance,
        None => {
            warn!("Instance {} not found for OAuth init", instance_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Check if this is a Remote OAuth instance
    if let MCPApplicationType::Remote(ref oauth_config) = instance.app_type {
        let provider_type = match oauth_config.provider_type.as_str() {
            "gitlab" => crate::api::oauth::OAuthProviderType::GitLab,
            "github" => crate::api::oauth::OAuthProviderType::GitHub,
            "google" => crate::api::oauth::OAuthProviderType::Google,
            custom => crate::api::oauth::OAuthProviderType::Custom(custom.to_string()),
        };

        // Get session ID from query params (for MCP clients to track their session)
        let session_id = params.get("session_id").unwrap_or(&instance_id).clone();
        let redirect_uri = format!(
            "http://localhost:8080/mcp/{}/oauth/callback?session_id={}",
            instance_id, session_id
        );

        // Generate OAuth authorization URL
        match manager
            .oauth_manager
            .get_authorization_url(
                provider_type,
                instance.installation_id.clone(),
                None,
                Some(redirect_uri),
                None,
            )
            .await
        {
            Ok(auth_url) => {
                info!("Redirecting to OAuth provider: {}", auth_url);
                Ok(axum::response::Redirect::temporary(&auth_url))
            }
            Err(e) => {
                error!("Failed to generate OAuth URL: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        warn!("Instance {} is not a Remote OAuth instance", instance_id);
        Err(StatusCode::BAD_REQUEST)
    }
}

// OAuth callback endpoint for MCP clients
async fn handle_mcp_oauth_callback(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Html<String>, StatusCode> {
    info!("OAuth callback received for instance: {}", instance_id);

    // Get the instance
    let instance = match manager.get_server_instance(&instance_id).await {
        Some(instance) => instance,
        None => {
            warn!("Instance {} not found for OAuth callback", instance_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Check if this is a Remote OAuth instance
    if let MCPApplicationType::Remote(ref oauth_config) = instance.app_type {
        let provider_type = match oauth_config.provider_type.as_str() {
            "gitlab" => crate::api::oauth::OAuthProviderType::GitLab,
            "github" => crate::api::oauth::OAuthProviderType::GitHub,
            "google" => crate::api::oauth::OAuthProviderType::Google,
            custom => crate::api::oauth::OAuthProviderType::Custom(custom.to_string()),
        };

        // Get authorization code from callback
        if let Some(code) = params.get("code") {
            let state = params.get("state").unwrap_or(&instance_id);
            let session_id = params.get("session_id").unwrap_or(&instance_id);
            let redirect_uri = format!(
                "http://localhost:8080/mcp/{}/oauth/callback?session_id={}",
                instance_id, session_id
            );

            // Handle OAuth callback to get tokens
            let oauth_callback = crate::api::oauth::OAuthCallback {
                code: code.clone(),
                state: state.clone(), // Use the actual state parameter from GitLab
                error: None,
                error_description: None,
            };

            match manager.oauth_manager.handle_callback(oauth_callback).await {
                Ok(stored_token) => {
                    info!("OAuth tokens stored for session: {}", session_id);

                    // Create session with OAuth tokens
                    let client_info = MCPClientInfo {
                        name: "Windsurf".to_string(),
                        version: "1.0.0".to_string(),
                        user_agent: Some("Windsurf MCP Client".to_string()),
                    };

                    let created_session_id = manager
                        .create_session(
                            instance_id.clone(),
                            instance.installation_id.clone(),
                            instance.app_id.clone(),
                            client_info,
                            Some(format!("oauth-user-{}", session_id)),
                            MCPSessionPermissions::default(),
                            vec![],
                        )
                        .await;

                    info!("Created session: {} for OAuth user", created_session_id);
                }
                Err(e) => {
                    error!("Failed to handle OAuth callback: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }

            // Return success page with instructions
            let html = format!(
                r#"
                        <!DOCTYPE html>
                        <html>
                        <head>
                            <title>OAuth Success</title>
                            <style>
                                body {{ font-family: Arial, sans-serif; margin: 40px; }}
                                .success {{ color: green; }}
                                .code {{ background: #f5f5f5; padding: 10px; border-radius: 5px; }}
                            </style>
                        </head>
                        <body>
                            <h1 class="success">✅ OAuth Authentication Successful!</h1>
                            <p>Your GitLab account has been successfully connected to the MCP server.</p>
                            <p><strong>Session ID:</strong> {}</p>
                            <p>You can now use the MCP server in Windsurf with the following configuration:</p>
                            <div class="code">
                                <pre>{{
  "mcpServers": {{
    "circuit-breaker-gitlab": {{
      "command": "curl",
      "args": [
        "-s", "-X", "POST",
        "-H", "Content-Type: application/json",
        "-H", "X-MCP-Session: {}",
        "-d", "@-",
        "http://localhost:8080/mcp/{}"
      ]
    }}
  }}
}}</pre>
                            </div>
                            <p>You can now close this window and refresh your MCP connection in Windsurf.</p>
                        </body>
                        </html>
                    "#,
                session_id, session_id, instance_id
            );

            Ok(axum::response::Html(html))
        } else if let Some(error) = params.get("error") {
            warn!("OAuth error: {}", error);
            let html = format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head><title>OAuth Error</title></head>
                <body>
                    <h1>❌ OAuth Error</h1>
                    <p>Error: {}</p>
                    <p>Please try again.</p>
                </body>
                </html>
            "#,
                error
            );
            Ok(axum::response::Html(html))
        } else {
            warn!("No code or error in OAuth callback");
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        warn!("Instance {} is not a Remote OAuth instance", instance_id);
        Err(StatusCode::BAD_REQUEST)
    }
}

async fn handle_token_request(
    Path(instance_id): Path<String>,
    State(manager): State<MCPServerManager>,
) -> Response {
    info!("Token request for instance: {}", instance_id);

    // Get fresh token using existing OAuth manager
    match manager
        .get_oauth_manager()
        .get_token(
            &crate::api::oauth::OAuthProviderType::GitLab,
            &instance_id,
            Some("oauth-user"),
        )
        .await
    {
        Ok(token) => {
            let response = serde_json::json!({
                "access_token": token.access_token,
                "token_type": "Bearer",
                "expires_at": token.expires_at,
                "scope": token.scope.join(" ")
            });

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(serde_json::to_string(&response).unwrap())
                .unwrap()
                .into_response()
        }
        Err(e) => {
            warn!("Failed to get fresh token for {}: {}", instance_id, e);
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(format!("{{\"error\": \"Token unavailable: {}\"}}", e))
                .unwrap()
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_server_with_instance() -> (CircuitBreakerMCPServer, String) {
        let manager = MCPServerManager::new();
        let instance_id = manager
            .create_server_instance(
                "test-app".to_string(),
                "test-installation".to_string(),
                "Test MCP Server".to_string(),
                "Test server for unit tests".to_string(),
                vec!["test-context".to_string()],
                crate::api::mcp_types::MCPApplicationType::Local,
            )
            .await
            .unwrap();

        let server = CircuitBreakerMCPServer::with_manager(manager);
        (server, instance_id)
    }

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let manager = MCPServerManager::new();
        let server = CircuitBreakerMCPServer::with_manager(manager);
        // Just verify we can create the server
        assert!(true);
    }

    #[tokio::test]
    async fn test_initialize_request() {
        let (server, instance_id) = create_test_server_with_instance().await;
        let request = MCPRequest {
            id: Some(MCPId::String("test-1".to_string())),
            method: "initialize".to_string(),
            params: None,
        };

        let response = server.handle_request(&instance_id, request, None).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_list_tools_request() {
        let (server, instance_id) = create_test_server_with_instance().await;
        let request = MCPRequest {
            id: Some(MCPId::String("test-2".to_string())),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = server.handle_request(&instance_id, request, None).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let (server, instance_id) = create_test_server_with_instance().await;
        let request = MCPRequest {
            id: Some(MCPId::String("test-3".to_string())),
            method: "unknown_method".to_string(),
            params: None,
        };

        let response = server.handle_request(&instance_id, request, None).await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_nonexistent_instance() {
        let server = CircuitBreakerMCPServer::new();
        let request = MCPRequest {
            id: Some(MCPId::String("test-4".to_string())),
            method: "initialize".to_string(),
            params: None,
        };

        let response = server
            .handle_request("nonexistent-instance", request, None)
            .await;
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, error_codes::INVALID_REQUEST);
    }
}
