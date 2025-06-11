// MCP (Model Context Protocol) types and schemas
// This module defines the core types for multi-tenant MCP server functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// MCP Server Instance information - represents a single MCP server instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerInstance {
    pub instance_id: String,
    pub app_id: String,
    pub installation_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: MCPCapabilities,
    pub project_contexts: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: MCPServerStatus,
}

/// MCP Server Registry - manages multiple MCP server instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerRegistry {
    pub servers: HashMap<String, MCPServerInstance>,
    pub installations: HashMap<String, MCPInstallation>,
    pub apps: HashMap<String, MCPApp>,
}

/// MCP Server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPServerStatus {
    Active,
    Suspended,
    Inactive,
}

/// MCP Server information (for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: MCPCapabilities,
}

/// MCP Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPCapabilities {
    pub tools: bool,
    pub prompts: bool,
    pub resources: bool,
    pub logging: bool,
}

/// MCP Request wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// MCP Response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
}

/// MCP Error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolCall {
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

/// MCP Tool call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolResult {
    pub content: Vec<MCPContent>,
    pub is_error: Option<bool>,
}

/// MCP Content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MCPContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: MCPResource },
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<MCPPromptArgument>,
}

/// MCP Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// MCP Session information - tied to a specific server instance and user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSession {
    pub session_id: String,
    pub server_instance_id: String,
    pub installation_id: String,
    pub app_id: String,
    pub user_id: Option<String>,
    pub client_info: MCPClientInfo,
    pub permissions: MCPSessionPermissions,
    pub project_contexts: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// MCP Session permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSessionPermissions {
    pub tools: Vec<String>,
    pub prompts: Vec<String>,
    pub resources: Vec<String>,
    pub project_contexts: HashMap<String, ProjectContextPermission>,
}

/// MCP Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPClientInfo {
    pub name: String,
    pub version: String,
    pub user_agent: Option<String>,
}

/// MCP standard error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_RANGE_START: i32 = -32099;
    pub const SERVER_ERROR_RANGE_END: i32 = -32000;
}

/// Helper functions for creating MCP responses
impl MCPResponse {
    pub fn success(id: String, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: String, code: i32, message: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(MCPError {
                code,
                message,
                data: None,
            }),
        }
    }

    pub fn error_with_data(
        id: String,
        code: i32,
        message: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id,
            result: None,
            error: Some(MCPError {
                code,
                message,
                data: Some(data),
            }),
        }
    }
}

/// Helper functions for creating MCP content
impl MCPContent {
    pub fn text(text: String) -> Self {
        Self::Text { text }
    }

    pub fn image(data: String, mime_type: String) -> Self {
        Self::Image { data, mime_type }
    }

    pub fn resource(resource: MCPResource) -> Self {
        Self::Resource { resource }
    }
}

/// Default capabilities for Circuit Breaker MCP Server
impl Default for MCPCapabilities {
    fn default() -> Self {
        Self {
            tools: true,
            prompts: true,
            resources: true,
            logging: true,
        }
    }
}

/// MCP App definition (similar to GitHub Apps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPApp {
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub homepage_url: Option<String>,
    pub webhook_url: Option<String>,
    pub permissions: MCPPermissions,
    pub events: Vec<String>,
    pub private_key: String,
    pub public_key: String,
    pub client_id: String,
    pub client_secret: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MCP Installation - represents an app installed in a specific context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPInstallation {
    pub installation_id: String,
    pub app_id: String,
    pub account: MCPAccount,
    pub permissions: MCPPermissions,
    pub project_contexts: Vec<ProjectContext>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
    pub suspended_by: Option<String>,
}

/// MCP Account (user or organization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPAccount {
    pub id: String,
    pub login: String,
    pub account_type: MCPAccountType,
    pub avatar_url: Option<String>,
    pub url: Option<String>,
}

/// MCP Account type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPAccountType {
    User,
    Organization,
}

/// MCP Permissions structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPermissions {
    pub workflows: PermissionLevel,
    pub agents: PermissionLevel,
    pub functions: PermissionLevel,
    pub external_apis: PermissionLevel,
    pub webhooks: PermissionLevel,
    pub audit_logs: PermissionLevel,
    pub project_contexts: Vec<ProjectContextPermission>,
}

/// Project Context definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub context_id: String,
    pub name: String,
    pub description: Option<String>,
    pub context_type: ProjectContextType,
    pub configuration: ProjectContextConfig,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Project Context type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectContextType {
    GitLab {
        project_id: String,
        namespace: String,
    },
    GitHub {
        owner: String,
        repo: String,
    },
    Combined {
        contexts: Vec<String>,
    },
    Custom {
        provider: String,
        identifier: String,
    },
}

/// Project Context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextConfig {
    pub base_url: Option<String>,
    pub api_version: Option<String>,
    pub default_branch: Option<String>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_depth: Option<u32>,
    pub cache_duration_hours: Option<u32>,
}

/// Project Context permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextPermission {
    pub context_id: String,
    pub context_type: ProjectContextType,
    pub permissions: ContextPermissions,
    pub resource_limits: ResourceLimits,
}

/// Context permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPermissions {
    pub read: bool,
    pub write: bool,
    pub admin: bool,
    pub allowed_operations: Vec<String>,
    pub restricted_paths: Vec<String>,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_file_size_mb: Option<u32>,
    pub max_search_results: Option<u32>,
    pub rate_limit_per_hour: Option<u32>,
    pub allowed_file_extensions: Vec<String>,
}

/// Permission level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionLevel {
    None,
    Read,
    Write,
    Admin,
}

/// Default server info for Circuit Breaker
impl Default for MCPServerInfo {
    fn default() -> Self {
        Self {
            name: "circuit-breaker-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Circuit Breaker MCP Server - Intelligent workflow automation and agent coordination".to_string(),
            capabilities: MCPCapabilities::default(),
        }
    }
}

/// Default permissions
impl Default for MCPPermissions {
    fn default() -> Self {
        Self {
            workflows: PermissionLevel::Read,
            agents: PermissionLevel::Read,
            functions: PermissionLevel::None,
            external_apis: PermissionLevel::None,
            webhooks: PermissionLevel::None,
            audit_logs: PermissionLevel::None,
            project_contexts: Vec::new(),
        }
    }
}

/// Default session permissions
impl Default for MCPSessionPermissions {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            prompts: Vec::new(),
            resources: Vec::new(),
            project_contexts: HashMap::new(),
        }
    }
}

/// Registry implementation
impl MCPServerRegistry {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            installations: HashMap::new(),
            apps: HashMap::new(),
        }
    }

    pub fn add_server_instance(&mut self, instance: MCPServerInstance) {
        self.servers.insert(instance.instance_id.clone(), instance);
    }

    pub fn get_server_instance(&self, instance_id: &str) -> Option<&MCPServerInstance> {
        self.servers.get(instance_id)
    }

    pub fn get_servers_for_installation(&self, installation_id: &str) -> Vec<&MCPServerInstance> {
        self.servers
            .values()
            .filter(|server| server.installation_id == installation_id)
            .collect()
    }

    pub fn add_installation(&mut self, installation: MCPInstallation) {
        self.installations
            .insert(installation.installation_id.clone(), installation);
    }

    pub fn get_installation(&self, installation_id: &str) -> Option<&MCPInstallation> {
        self.installations.get(installation_id)
    }

    pub fn add_app(&mut self, app: MCPApp) {
        self.apps.insert(app.app_id.clone(), app);
    }

    pub fn get_app(&self, app_id: &str) -> Option<&MCPApp> {
        self.apps.get(app_id)
    }
}

impl Default for MCPServerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_response_creation() {
        let success_response =
            MCPResponse::success("test-id".to_string(), serde_json::json!({"status": "ok"}));
        assert!(success_response.result.is_some());
        assert!(success_response.error.is_none());

        let error_response = MCPResponse::error(
            "test-id".to_string(),
            error_codes::INVALID_REQUEST,
            "Invalid request".to_string(),
        );
        assert!(error_response.result.is_none());
        assert!(error_response.error.is_some());
    }

    #[test]
    fn test_mcp_content_creation() {
        let text_content = MCPContent::text("Hello world".to_string());
        match text_content {
            MCPContent::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_default_capabilities() {
        let caps = MCPCapabilities::default();
        assert!(caps.tools);
        assert!(caps.prompts);
        assert!(caps.resources);
        assert!(caps.logging);
    }
}
