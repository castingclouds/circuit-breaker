// MCP (Model Context Protocol) server implementation
// This module implements the multi-tenant MCP server functionality for Circuit Breaker

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::mcp_auth::{ClientInfo, MCPJWTService, MCPTokenClaims};
use super::mcp_types::*;

/// Circuit Breaker MCP Server Manager - manages multiple MCP server instances
#[derive(Clone)]
pub struct MCPServerManager {
    pub registry: Arc<RwLock<MCPServerRegistry>>,
    pub sessions: Arc<RwLock<HashMap<String, MCPSession>>>,
    pub jwt_service: Arc<MCPJWTService>,
}

impl MCPServerManager {
    /// Create a new MCP server manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(MCPServerRegistry::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            jwt_service: Arc::new(MCPJWTService::new()),
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
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            status: MCPServerStatus::Active,
        };

        let mut registry = self.registry.write().await;
        registry.add_server_instance(instance);

        info!("Created new MCP server instance: {}", instance_id);
        Ok(instance_id)
    }

    /// Get server instance
    pub async fn get_server_instance(&self, instance_id: &str) -> Option<MCPServerInstance> {
        let registry = self.registry.read().await;
        registry.get_server_instance(instance_id).cloned()
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

    /// Get default tools for a server instance
    pub async fn get_default_tools(&self, _instance_id: &str) -> Vec<MCPTool> {
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

    /// Create the MCP router with multi-tenant support
    pub fn create_router(&self) -> Router {
        Router::new()
            // Multi-tenant MCP protocol endpoints
            .route("/mcp/:instance_id", post(handle_mcp_request))
            .route("/mcp/:instance_id/ws", get(handle_mcp_websocket))
            // Server instance management
            .route("/mcp/instances", post(create_mcp_instance))
            .route("/mcp/instances/:instance_id", get(get_mcp_instance))
            .route("/mcp/instances/:instance_id/info", get(get_server_info))
            // Tool management endpoints (per instance)
            .route("/mcp/:instance_id/tools", get(list_tools))
            .route("/mcp/:instance_id/prompts", get(list_prompts))
            .route("/mcp/:instance_id/resources", get(list_resources))
            // Authentication endpoints
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

        // Verify the instance exists
        let instance = match self.manager.get_server_instance(instance_id).await {
            Some(instance) => instance,
            None => {
                return MCPResponse::error(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    format!("Server instance '{}' not found", instance_id),
                );
            }
        };

        // Verify authentication and permissions if provided
        if let Some(claims) = &claims {
            // Verify the claims are valid for this instance
            if claims.installation_id != instance.installation_id {
                return MCPResponse::error(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    "Token not valid for this instance".to_string(),
                );
            }

            // Check if the app matches
            if claims.app_id != instance.app_id {
                return MCPResponse::error(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    "Token app_id does not match instance".to_string(),
                );
            }
        }

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request, &instance).await,
            "tools/list" => self.handle_list_tools(request, &instance).await,
            "tools/call" => self.handle_call_tool(request, &instance).await,
            "prompts/list" => self.handle_list_prompts(request, &instance).await,
            "prompts/get" => self.handle_get_prompt(request, &instance).await,
            "resources/list" => self.handle_list_resources(request, &instance).await,
            "resources/read" => self.handle_read_resource(request, &instance).await,
            _ => MCPResponse::error(
                request.id,
                error_codes::METHOD_NOT_FOUND,
                format!("Method '{}' not found", request.method),
            ),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Initializing MCP server instance: {}", instance.instance_id);

        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": instance.capabilities,
            "serverInfo": {
                "name": instance.name,
                "version": env!("CARGO_PKG_VERSION"),
                "instanceId": instance.instance_id,
                "installationId": instance.installation_id,
                "appId": instance.app_id
            }
        });

        MCPResponse::success(request.id, result)
    }

    /// Handle list tools request
    async fn handle_list_tools(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!(
            "Listing available tools for instance: {}",
            instance.instance_id
        );

        let tools = self.manager.get_default_tools(&instance.instance_id).await;
        let result = serde_json::json!({
            "tools": tools
        });

        MCPResponse::success(request.id, result)
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
                return MCPResponse::error(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    "Missing tool call parameters".to_string(),
                );
            }
        };

        let tool_call: MCPToolCall = match serde_json::from_value(params) {
            Ok(call) => call,
            Err(e) => {
                return MCPResponse::error(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    format!("Invalid tool call parameters: {}", e),
                );
            }
        };

        // Execute the tool in the context of this instance
        let result = self.execute_tool(tool_call, instance).await;

        match result {
            Ok(tool_result) => {
                MCPResponse::success(request.id, serde_json::to_value(tool_result).unwrap())
            }
            Err(e) => MCPResponse::error(
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

        MCPResponse::success(request.id, result)
    }

    /// Handle get prompt request
    async fn handle_get_prompt(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Getting prompt for instance: {}", instance.instance_id);

        // For now, return a placeholder response
        MCPResponse::success(
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

        MCPResponse::success(request.id, result)
    }

    /// Handle read resource request
    async fn handle_read_resource(
        &self,
        request: MCPRequest,
        instance: &MCPServerInstance,
    ) -> MCPResponse {
        debug!("Reading resource for instance: {}", instance.instance_id);

        // For now, return a placeholder response
        MCPResponse::success(
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

                // TODO: Check if the authenticated user has permission to access this context
                // This would involve checking the claims.permissions.project_contexts

                Ok(MCPToolResult {
                    content: vec![MCPContent::text(format!(
                        "Search completed for context {} in instance {} (placeholder)",
                        context_id, instance.instance_id
                    ))],
                    is_error: Some(false),
                })
            }
            _ => Err(format!("Unknown tool: {}", tool_call.name).into()),
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
async fn handle_mcp_request(
    State(manager): State<MCPServerManager>,
    Path(instance_id): Path<String>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<MCPRequest>,
) -> Result<axum::Json<MCPResponse>, StatusCode> {
    // Attempt authentication (required for most operations except initialize)
    let claims = if request.method == "initialize" {
        // Initialize doesn't require authentication
        None
    } else {
        // All other operations require authentication
        match manager.authenticate_request(&headers).await {
            Ok(claims) => Some(claims),
            Err(e) => {
                warn!(
                    "Authentication failed for request to {}: {}",
                    instance_id, e
                );
                return Ok(axum::Json(MCPResponse::error(
                    request.id,
                    error_codes::INVALID_REQUEST,
                    format!("Authentication failed: {}", e),
                )));
            }
        }
    };

    let server = CircuitBreakerMCPServer { manager };
    let response = server.handle_request(&instance_id, request, claims).await;
    Ok(axum::Json(response))
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
        let error_response = MCPResponse::error(
            "init".to_string(),
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
                        let error_response = MCPResponse::error(
                            "unknown".to_string(),
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
            id: "test-1".to_string(),
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
            id: "test-2".to_string(),
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
            id: "test-3".to_string(),
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
            id: "test-4".to_string(),
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
