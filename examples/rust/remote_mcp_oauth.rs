//! MCP CLI - Command Line Interface for Multi-Context Protocol Server
//!
//! This CLI provides a comprehensive interface for interacting with the MCP server,
//! including authentication, OAuth provider management, and server operations.
//!
//! ## Complete MCP Demo Workflow
//!
//! This example demonstrates a complete multi-tenant remote MCP server setup:
//! 1. Server health check and status
//! 2. MCP app creation and installation
//! 3. OAuth provider registration (GitLab)
//! 4. Browser-based OAuth authentication flow
//! 5. GitLab API integration testing
//! 6. Project context discovery
//! 7. Issue management and user information retrieval

use anyhow::{Context, Result};
use axum::{extract::Query, response::Html, routing::get, Router, Server};
use base64::{self, Engine};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// CLI Configuration stored in user's home directory
#[derive(Debug, Serialize, Deserialize, Default)]
struct CliConfig {
    server_url: Option<String>,
    current_session: Option<String>,
    sessions: HashMap<String, SessionInfo>,
    oauth_tokens: HashMap<String, OAuthTokenInfo>,
}

/// Session information for authentication
#[derive(Debug, Serialize, Deserialize, Clone)]
struct SessionInfo {
    session_id: String,
    jwt_token: String,
    installation_id: Option<String>,
    app_id: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

/// OAuth token information
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OAuthTokenInfo {
    provider_type: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    scope: Vec<String>,
    created_at: DateTime<Utc>,
}

/// Server status response
#[derive(Debug, Deserialize)]
struct ServerStatus {
    status: String,
    version: String,
    uptime: String,
    active_sessions: u32,
    registered_apps: u32,
}

/// OAuth provider registration request
#[derive(Debug, Serialize)]
struct OAuthProviderRequest {
    provider_type: String,
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    scope: Vec<String>,
    redirect_uri: String,
}

/// OAuth authorization response
#[derive(Debug, Deserialize)]
struct OAuthAuthResponse {
    auth_url: String,
    state: String,
}

/// OAuth callback parameters
#[derive(Debug, Deserialize)]
struct OAuthCallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Shared state for OAuth callback
#[derive(Debug, Clone)]
struct CallbackResult {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// JWT Claims for app authentication
#[derive(Debug, Serialize)]
struct AppJWTClaims {
    iss: String, // app_id
    iat: i64,    // issued at
    exp: i64,    // expires at
    aud: String, // audience - "mcp-server"
}

/// Remote MCP OAuth Demo Application
#[derive(Parser)]
#[command(name = "remote-mcp-oauth")]
#[command(about = "Remote Multi-Context Protocol Server with OAuth Demo")]
#[command(version = "0.1.0")]
struct Cli {
    /// Server URL (can also be set via MCP_SERVER_URL env var)
    #[arg(long, env = "MCP_SERVER_URL", default_value = "http://localhost:8080")]
    server_url: String,

    /// Configuration file path
    #[arg(long, default_value = "~/.remote-mcp-oauth.json")]
    config: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Server management commands
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// OAuth provider management
    #[command(name = "oauth")]
    OAuth {
        #[command(subcommand)]
        action: OAuthAction,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Complete MCP Demo Workflow
    Demo {
        #[command(subcommand)]
        action: DemoAction,
    },
    /// Interactive mode
    Interactive,
}

#[derive(Subcommand)]
enum ServerAction {
    /// Check server status
    Status,
    /// List server instances
    List,
    /// Get server information
    Info {
        /// Server instance ID
        #[arg(short, long)]
        instance_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthAction {
    /// Create a new MCP app
    CreateApp {
        /// App name
        #[arg(short, long)]
        name: String,
        /// App description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Install an app
    Install {
        /// App ID
        #[arg(short, long)]
        app_id: String,
        /// Installation context
        #[arg(short, long)]
        context: Option<String>,
    },
    /// Login with JWT
    Login {
        /// App ID
        #[arg(short, long)]
        app_id: String,
        /// Installation ID
        #[arg(short, long)]
        installation_id: Option<String>,
    },
    /// Logout (revoke session)
    Logout,
    /// Show current authentication status
    Status,
    /// List available apps
    ListApps,
}

#[derive(Subcommand)]
enum OAuthAction {
    /// Register an OAuth provider
    Register {
        /// Provider type (gitlab, github, google)
        #[arg(short, long)]
        provider: String,
        /// Client ID
        #[arg(long)]
        client_id: String,
        /// Client secret
        #[arg(long)]
        client_secret: String,
        /// Redirect URI
        #[arg(long)]
        redirect_uri: String,
        /// Scopes (comma-separated)
        #[arg(long)]
        scopes: Option<String>,
    },
    /// Start OAuth authorization flow
    Authorize {
        /// Provider type
        #[arg(short, long)]
        provider: String,
        /// Installation ID
        #[arg(short, long)]
        installation_id: String,
        /// Custom redirect URI (optional, defaults to local callback)
        #[arg(long)]
        redirect_uri: Option<String>,
    },
    /// Complete OAuth callback
    Callback {
        /// Authorization code
        #[arg(short, long)]
        code: String,
        /// State parameter
        #[arg(short, long)]
        state: String,
    },
    /// List OAuth tokens
    List,
    /// Revoke OAuth token
    Revoke {
        /// Provider type
        #[arg(short, long)]
        provider: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List active sessions
    List,
    /// Switch to a different session
    Switch {
        /// Session ID
        session_id: String,
    },
    /// Show current session details
    Current,
    /// Clear all sessions
    Clear,
}

#[derive(Subcommand)]
enum DemoAction {
    /// Run the complete MCP workflow demo
    Full {
        /// NgRok URL for the MCP server
        #[arg(long, env = "NGROK_URL")]
        ngrok_url: Option<String>,
        /// GitLab OAuth Client ID
        #[arg(long, env = "GITLAB_CLIENT_ID")]
        gitlab_client_id: Option<String>,
        /// GitLab OAuth Client Secret
        #[arg(long, env = "GITLAB_CLIENT_SECRET")]
        gitlab_client_secret: Option<String>,
        /// Skip confirmation prompts
        #[arg(long)]
        auto_confirm: bool,
    },
    /// Test GitLab integration only
    GitLab {
        /// NgRok URL for the MCP server
        #[arg(long, env = "NGROK_URL")]
        ngrok_url: Option<String>,
    },
    /// Setup OAuth provider only
    SetupOAuth {
        /// NgRok URL for the MCP server
        #[arg(long, env = "NGROK_URL")]
        ngrok_url: Option<String>,
        /// GitLab OAuth Client ID
        #[arg(long, env = "GITLAB_CLIENT_ID")]
        gitlab_client_id: Option<String>,
        /// GitLab OAuth Client Secret
        #[arg(long, env = "GITLAB_CLIENT_SECRET")]
        gitlab_client_secret: Option<String>,
    },
}

/// CLI Application State
struct CliApp {
    config: CliConfig,
    config_path: PathBuf,
    client: Client,
    server_url: String,
    verbose: bool,
}

impl CliApp {
    /// Create new CLI application
    fn new(server_url: String, config_path: String, verbose: bool) -> Result<Self> {
        let config_path = shellexpand::tilde(&config_path).into_owned();
        let config_path = PathBuf::from(config_path);

        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
            serde_json::from_str(&content).context("Failed to parse config file")?
        } else {
            CliConfig::default()
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            config_path,
            client,
            server_url,
            verbose,
        })
    }

    /// Save configuration to file
    fn save_config(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content =
            serde_json::to_string_pretty(&self.config).context("Failed to serialize config")?;

        fs::write(&self.config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get current session token
    fn get_current_token(&self) -> Option<&str> {
        self.config
            .current_session
            .as_ref()
            .and_then(|session_id| self.config.sessions.get(session_id))
            .map(|session| session.jwt_token.as_str())
    }

    /// Make authenticated request
    async fn make_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        self.make_request_with_app_auth(method, path, body, None)
            .await
    }

    /// Make authenticated request with optional app authentication
    async fn make_request_with_app_auth(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        app_id: Option<&str>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.server_url, path);
        let mut request = self
            .client
            .request(method, &url)
            .header("Content-Type", "application/json");

        // Use app JWT auth if app_id is provided
        if let Some(app_id) = app_id {
            let jwt_token = self.generate_app_jwt(app_id)?;
            request = request.bearer_auth(jwt_token);
        } else if let Some(token) = self.get_current_token() {
            request = request.bearer_auth(token);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.context("Failed to send request")?;

        if self.verbose {
            println!("Request: {} {}", response.status(), url);
        }

        Ok(response)
    }

    /// Start local OAuth callback server
    async fn start_oauth_callback_server(
        &self,
        port: u16,
    ) -> Result<Arc<Mutex<Option<CallbackResult>>>> {
        let callback_result = Arc::new(Mutex::new(None));
        let callback_result_clone = callback_result.clone();

        let app = Router::new()
            .route("/mcp/oauth/callback", get(move |query: Query<OAuthCallbackParams>| {
                let callback_result = callback_result_clone.clone();
                async move {
                    let params = query.0;

                    let result = CallbackResult {
                        code: params.code.clone(),
                        state: params.state.clone(),
                        error: params.error.clone(),
                    };

                    // Store the result
                    *callback_result.lock().await = Some(result);

                    // Return a nice HTML response
                    if params.error.is_some() {
                        Html(format!(
                            r#"
                            <html>
                            <head><title>OAuth Error</title></head>
                            <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
                                <h1 style="color: #e74c3c;">OAuth Authorization Failed</h1>
                                <p>Error: {}</p>
                                <p>Description: {}</p>
                                <p>You can close this window and return to the CLI.</p>
                            </body>
                            </html>
                            "#,
                            params.error.unwrap_or_default(),
                            params.error_description.unwrap_or_default()
                        ))
                    } else {
                        Html(format!(
                            r#"
                            <html>
                            <head><title>OAuth Success</title></head>
                            <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
                                <h1 style="color: #27ae60;">✅ OAuth Authorization Successful!</h1>
                                <p>Authorization code received: <code>{}</code></p>
                                <p>State: <code>{}</code></p>
                                <p>You can close this window and return to the CLI.</p>
                                <script>
                                    // Auto-close after 3 seconds
                                    setTimeout(function() {{
                                        window.close();
                                    }}, 3000);
                                </script>
                            </body>
                            </html>
                            "#,
                            params.code.as_deref().unwrap_or("N/A"),
                            params.state.as_deref().unwrap_or("N/A")
                        ))
                    }
                }
            }))
            .layer(CorsLayer::permissive());

        let addr = format!("127.0.0.1:{}", port);
        println!(
            "{}",
            format!("🌐 Starting local OAuth callback server on http://{}", addr).blue()
        );

        tokio::spawn(async move {
            axum::Server::bind(&addr.parse().unwrap())
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(callback_result)
    }

    /// Run the CLI application
    async fn run(mut self, command: Commands) -> Result<()> {
        match command {
            Commands::Server { action } => self.handle_server_action(action).await,
            Commands::Auth { action } => self.handle_auth_action(action).await,
            Commands::OAuth { action } => self.handle_oauth_action(action).await,
            Commands::Session { action } => self.handle_session_action(action).await,
            Commands::Demo { action } => self.handle_demo_action(action).await,
            Commands::Interactive => self.run_interactive().await,
        }
    }

    /// Handle server actions
    async fn handle_server_action(&mut self, action: ServerAction) -> Result<()> {
        match action {
            ServerAction::Status => {
                println!("{}", "Checking server status...".blue());

                // Try both GraphQL health endpoint and OpenAI health endpoint
                let graphql_health = self
                    .client
                    .get(&format!(
                        "{}:{}/health",
                        self.server_url.replace(":3000", ""),
                        4000
                    ))
                    .send()
                    .await;
                let openai_health = self
                    .client
                    .get(&format!("{}/health", self.server_url))
                    .send()
                    .await;

                match (graphql_health, openai_health) {
                    (Ok(graphql_resp), Ok(openai_resp))
                        if graphql_resp.status().is_success()
                            && openai_resp.status().is_success() =>
                    {
                        println!("{}", "Server Status:".green().bold());
                        println!("  GraphQL Server: {} (Port 4000)", "✅ Running".green());
                        println!("  OpenAI API Server: {} (Port 3000)", "✅ Running".green());
                        println!("  Status: {}", "Healthy".green());
                    }
                    (Ok(graphql_resp), _) if graphql_resp.status().is_success() => {
                        println!("{}", "Server Status:".yellow().bold());
                        println!("  GraphQL Server: {} (Port 4000)", "✅ Running".green());
                        println!(
                            "  OpenAI API Server: {} (Port 3000)",
                            "❌ Not Running".red()
                        );
                        println!("  Status: {}", "Partial".yellow());
                    }
                    (_, Ok(openai_resp)) if openai_resp.status().is_success() => {
                        println!("{}", "Server Status:".yellow().bold());
                        println!("  GraphQL Server: {} (Port 4000)", "❌ Not Running".red());
                        println!("  OpenAI API Server: {} (Port 3000)", "✅ Running".green());
                        println!("  Status: {}", "Partial".yellow());
                    }
                    _ => {
                        println!("{}", "Server Status:".red().bold());
                        println!("  GraphQL Server: {} (Port 4000)", "❌ Not Running".red());
                        println!(
                            "  OpenAI API Server: {} (Port 3000)",
                            "❌ Not Running".red()
                        );
                        println!("  Status: {}", "Offline".red());
                        println!("💡 Start the server with: cargo run --bin server");
                    }
                }
            }
            ServerAction::List => {
                println!("{}", "Listing server instances...".blue());

                let response = self
                    .make_request(reqwest::Method::GET, "/api/servers", None)
                    .await?;

                if response.status().is_success() {
                    let servers: Value = response.json().await?;
                    println!("{}", serde_json::to_string_pretty(&servers)?);
                } else {
                    println!(
                        "{}",
                        format!("Failed to list servers: {}", response.status()).red()
                    );
                }
            }
            ServerAction::Info { instance_id } => {
                let id = match instance_id {
                    Some(id) => id,
                    None => {
                        let id: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Enter server instance ID")
                            .interact_text()?;
                        id
                    }
                };

                let response = self
                    .make_request(reqwest::Method::GET, &format!("/api/servers/{}", id), None)
                    .await?;

                if response.status().is_success() {
                    let info: Value = response.json().await?;
                    println!("{}", serde_json::to_string_pretty(&info)?);
                } else {
                    println!(
                        "{}",
                        format!("Failed to get server info: {}", response.status()).red()
                    );
                }
            }
        }
        Ok(())
    }

    /// Handle authentication actions
    async fn handle_auth_action(&mut self, action: AuthAction) -> Result<()> {
        match action {
            AuthAction::CreateApp { name, description } => {
                println!("{}", format!("Creating app '{}'...", name).blue());

                // Generate app ID and keys for testing
                let app_id = format!(
                    "app_{}",
                    Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
                );
                let (private_key, public_key) = self.generate_test_keys();

                let body = serde_json::json!({
                    "app_id": app_id,
                    "name": name,
                    "description": description.unwrap_or_else(|| format!("CLI generated app: {}", name)),
                    "owner": "cli-user",
                    "homepage_url": null,
                    "webhook_url": null,
                    "permissions": {
                        "workflows": "Read",
                        "agents": "Read",
                        "functions": "None",
                        "external_apis": "None",
                        "webhooks": "None",
                        "audit_logs": "None",
                        "project_contexts": []
                    },
                    "events": [],
                    "private_key": private_key,
                    "public_key": public_key,
                    "client_id": format!("client_{}", &app_id[4..]),
                    "client_secret": format!("secret_{}", Uuid::new_v4().to_string().replace("-", "")),
                    "created_at": chrono::Utc::now().to_rfc3339(),
                    "updated_at": chrono::Utc::now().to_rfc3339()
                });

                let response = self
                    .make_request(reqwest::Method::POST, "/mcp/auth/apps", Some(body))
                    .await?;

                if response.status().is_success() {
                    let app: Value = response.json().await?;
                    println!("{}", "App created successfully!".green());
                    println!("App ID: {}", app_id.yellow());
                    println!("{}", serde_json::to_string_pretty(&app)?);
                } else {
                    let error_text = response.text().await?;
                    println!("{}", format!("Failed to create app: {}", error_text).red());
                }
            }
            AuthAction::Install { app_id, context } => {
                println!("{}", format!("Installing app '{}'...", app_id).blue());

                let installation_id = format!(
                    "inst_{}",
                    Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
                );

                let body = serde_json::json!({
                    "installation_id": installation_id,
                    "app_id": app_id,
                    "account": {
                        "id": "user_123",
                        "login": "cli-user",
                        "account_type": "User"
                    },
                    "permissions": {
                        "workflows": "Read",
                        "agents": "Read",
                        "functions": "None",
                        "external_apis": "None",
                        "webhooks": "None",
                        "audit_logs": "None",
                        "project_contexts": []
                    },
                    "project_contexts": context.map(|ctx| {
                        if ctx.starts_with("gitlab:") {
                            // Parse GitLab context: gitlab:owner/repo
                            let gitlab_path = ctx.strip_prefix("gitlab:").unwrap_or(&ctx);
                            let parts: Vec<&str> = gitlab_path.split('/').collect();
                            if parts.len() >= 2 {
                                let owner = parts[0];
                                let repo = parts[1];
                                vec![serde_json::json!({
                                    "context_id": format!("gitlab_{}_{}", owner, repo),
                                    "name": format!("{}/{}", owner, repo),
                                    "description": format!("GitLab repository: {}/{}", owner, repo),
                                    "context_type": {
                                        "GitLab": {
                                            "project_id": format!("{}/{}", owner, repo),
                                            "namespace": owner
                                        }
                                    },
                                    "configuration": {
                                        "base_url": "https://gitlab.com",
                                        "api_version": "v4",
                                        "default_branch": "main",
                                        "include_patterns": ["**/*"],
                                        "exclude_patterns": [".git/**", "target/**", "node_modules/**"],
                                        "max_depth": 10,
                                        "cache_duration_hours": 24
                                    },
                                    "metadata": {
                                        "repository_type": "gitlab",
                                        "owner": owner,
                                        "repo": repo,
                                        "full_name": format!("{}/{}", owner, repo)
                                    },
                                    "created_at": chrono::Utc::now().to_rfc3339(),
                                    "last_accessed": null
                                })]
                            } else {
                                vec![serde_json::json!({
                                    "context_id": format!("ctx_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
                                    "name": ctx,
                                    "description": format!("Invalid GitLab context: {}", ctx),
                                    "context_type": {
                                        "Custom": {
                                            "provider": "gitlab",
                                            "identifier": ctx
                                        }
                                    },
                                    "configuration": {
                                        "base_url": null,
                                        "api_version": null,
                                        "default_branch": null,
                                        "include_patterns": [],
                                        "exclude_patterns": [],
                                        "max_depth": null,
                                        "cache_duration_hours": null
                                    },
                                    "metadata": {},
                                    "created_at": chrono::Utc::now().to_rfc3339(),
                                    "last_accessed": null
                                })]
                            }
                        } else if ctx.starts_with("github:") {
                            // Parse GitHub context: github:owner/repo
                            let github_path = ctx.strip_prefix("github:").unwrap_or(&ctx);
                            let parts: Vec<&str> = github_path.split('/').collect();
                            if parts.len() >= 2 {
                                let owner = parts[0];
                                let repo = parts[1];
                                vec![serde_json::json!({
                                    "context_id": format!("github_{}_{}", owner, repo),
                                    "name": format!("{}/{}", owner, repo),
                                    "description": format!("GitHub repository: {}/{}", owner, repo),
                                    "context_type": {
                                        "GitHub": {
                                            "owner": owner,
                                            "repo": repo
                                        }
                                    },
                                    "configuration": {
                                        "base_url": "https://api.github.com",
                                        "api_version": "2022-11-28",
                                        "default_branch": "main",
                                        "include_patterns": ["**/*"],
                                        "exclude_patterns": [".git/**", "target/**", "node_modules/**"],
                                        "max_depth": 10,
                                        "cache_duration_hours": 24
                                    },
                                    "metadata": {
                                        "repository_type": "github",
                                        "owner": owner,
                                        "repo": repo,
                                        "full_name": format!("{}/{}", owner, repo)
                                    },
                                    "created_at": chrono::Utc::now().to_rfc3339(),
                                    "last_accessed": null
                                })]
                            } else {
                                vec![serde_json::json!({
                                    "context_id": format!("ctx_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
                                    "name": ctx,
                                    "description": format!("Invalid GitHub context: {}", ctx),
                                    "context_type": {
                                        "Custom": {
                                            "provider": "github",
                                            "identifier": ctx
                                        }
                                    },
                                    "configuration": {
                                        "base_url": null,
                                        "api_version": null,
                                        "default_branch": null,
                                        "include_patterns": [],
                                        "exclude_patterns": [],
                                        "max_depth": null,
                                        "cache_duration_hours": null
                                    },
                                    "metadata": {},
                                    "created_at": chrono::Utc::now().to_rfc3339(),
                                    "last_accessed": null
                                })]
                            }
                        } else {
                            // Default custom context
                            vec![serde_json::json!({
                                "context_id": format!("ctx_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
                                "name": ctx,
                                "description": format!("Custom context: {}", ctx),
                                "context_type": {
                                    "Custom": {
                                        "provider": "cli",
                                        "identifier": ctx
                                    }
                                },
                                "configuration": {
                                    "base_url": null,
                                    "api_version": null,
                                    "default_branch": null,
                                    "include_patterns": [],
                                    "exclude_patterns": [],
                                    "max_depth": null,
                                    "cache_duration_hours": null
                                },
                                "metadata": {},
                                "created_at": chrono::Utc::now().to_rfc3339(),
                                "last_accessed": null
                            })]
                        }
                    }).unwrap_or_else(Vec::new),
                    "created_at": chrono::Utc::now().to_rfc3339(),
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                    "suspended_at": null,
                    "suspended_by": null
                });

                let response = self
                    .make_request(reqwest::Method::POST, "/mcp/auth/installations", Some(body))
                    .await?;

                if response.status().is_success() {
                    let installation: Value = response.json().await?;
                    println!("{}", "App installed successfully!".green());
                    println!("Installation ID: {}", installation_id.yellow());
                    println!("{}", serde_json::to_string_pretty(&installation)?);
                } else {
                    let error_text = response.text().await?;
                    println!("{}", format!("Failed to install app: {}", error_text).red());
                }
            }
            AuthAction::Login {
                app_id,
                installation_id,
            } => {
                println!("{}", "Creating authentication token...".blue());

                if let Some(inst_id) = installation_id {
                    // Create installation token
                    let body = serde_json::json!({
                        "permissions": {
                            "workflows": "Read",
                            "agents": "Read",
                            "functions": "None",
                            "external_apis": "None",
                            "webhooks": "None",
                            "audit_logs": "None",
                            "project_contexts": []
                        }
                    });

                    let response = self
                        .make_request_with_app_auth(
                            reqwest::Method::POST,
                            &format!("/mcp/auth/installations/{}/tokens", inst_id),
                            Some(body),
                            Some(&app_id), // Use the provided app_id
                        )
                        .await?;

                    if response.status().is_success() {
                        let token_response: Value = response.json().await?;

                        if let Some(token) = token_response.get("token").and_then(|t| t.as_str()) {
                            let session_info = SessionInfo {
                                session_id: format!(
                                    "session_{}",
                                    Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
                                ),
                                jwt_token: token.to_string(),
                                installation_id: Some(inst_id.clone()),
                                app_id: app_id.clone(),
                                expires_at: Utc::now() + chrono::Duration::hours(24), // Default expiry
                                created_at: Utc::now(),
                            };

                            let session_id = session_info.session_id.clone();
                            self.config
                                .sessions
                                .insert(session_id.clone(), session_info);
                            self.config.current_session = Some(session_id.clone());
                            self.save_config()?;

                            println!("{}", "Token created successfully!".green());
                            println!("Session ID: {}", session_id.yellow());
                            println!("Installation ID: {}", inst_id.yellow());
                        } else {
                            println!("{}", "Token response missing token field".red());
                        }
                    } else {
                        let error_text = response.text().await?;
                        println!(
                            "{}",
                            format!("Failed to create token: {}", error_text).red()
                        );
                    }
                } else {
                    println!("{}", "Installation ID is required for token creation".red());
                }
            }
            AuthAction::Logout => {
                if let Some(session_id) = &self.config.current_session {
                    if let Some(session) = self.config.sessions.get(session_id) {
                        // Try to extract token ID from JWT for revocation
                        // For now, just clear local session
                        println!("{}", "Clearing local session...".blue());

                        self.config.sessions.remove(session_id);
                        self.config.current_session = None;
                        self.save_config()?;
                        println!("{}", "Session cleared successfully!".green());
                        println!(
                            "{}",
                            "Note: Use token revocation for server-side cleanup".yellow()
                        );
                    }
                } else {
                    println!("{}", "No active session to clear".yellow());
                }
            }
            AuthAction::Status => {
                if let Some(session_id) = &self.config.current_session {
                    if let Some(session) = self.config.sessions.get(session_id) {
                        println!("{}", "Authentication Status:".green().bold());
                        println!("  Session ID: {}", session.session_id.yellow());
                        println!("  App ID: {}", session.app_id);
                        if let Some(inst_id) = &session.installation_id {
                            println!("  Installation ID: {}", inst_id);
                        }
                        println!(
                            "  Created: {}",
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        println!(
                            "  Expires: {}",
                            session.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );

                        if session.expires_at < Utc::now() {
                            println!("  {}", "Status: EXPIRED".red().bold());
                        } else {
                            println!("  {}", "Status: ACTIVE".green().bold());
                        }
                    }
                } else {
                    println!("{}", "Not authenticated".yellow());
                }
            }
            AuthAction::ListApps => {
                println!("{}", "Note: App listing not available via API".yellow());
                println!("{}", "Local sessions:".green().bold());

                if self.config.sessions.is_empty() {
                    println!("  {}", "No active sessions".yellow());
                } else {
                    for (session_id, session) in &self.config.sessions {
                        println!("  Session: {}", session_id.blue());
                        println!("    App ID: {}", session.app_id);
                        if let Some(inst_id) = &session.installation_id {
                            println!("    Installation ID: {}", inst_id);
                        }
                        println!(
                            "    Created: {}",
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        println!(
                            "    Expires: {}",
                            session.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        println!();
                    }
                }
            }
        }
        Ok(())
    }

    /// Generate test RSA key pair for app creation
    fn generate_test_keys(&self) -> (String, String) {
        // Read actual RSA keys from test_keys directory
        let private_key = std::fs::read_to_string("test_keys/test.pem")
            .expect("Failed to read test_keys/test.pem");
        let public_key = std::fs::read_to_string("test_keys/test.pub")
            .expect("Failed to read test_keys/test.pub");
        (private_key, public_key)
    }

    /// Generate JWT token for app authentication
    fn generate_app_jwt(&self, app_id: &str) -> Result<String> {
        let private_key = std::fs::read_to_string("test_keys/test.pem")
            .context("Failed to read app private key")?;

        let encoding_key = EncodingKey::from_rsa_pem(private_key.as_bytes())
            .context("Failed to parse private key")?;

        let now = chrono::Utc::now().timestamp();
        let claims = AppJWTClaims {
            iss: app_id.to_string(),
            iat: now,
            exp: now + 3600, // 1 hour expiry
            aud: "mcp-server".to_string(),
        };

        let header = Header::new(Algorithm::RS256);
        let token = encode(&header, &claims, &encoding_key).context("Failed to encode JWT")?;

        Ok(token)
    }

    /// Handle OAuth actions
    async fn handle_oauth_action(&mut self, action: OAuthAction) -> Result<()> {
        match action {
            OAuthAction::Register {
                provider,
                client_id,
                client_secret,
                redirect_uri,
                scopes,
            } => {
                println!(
                    "{}",
                    format!("Registering {} OAuth provider...", provider).blue()
                );

                let scopes_vec = scopes
                    .map(|s| s.split(',').map(|scope| scope.trim().to_string()).collect())
                    .unwrap_or_default();

                let (auth_url, token_url) = match provider.as_str() {
                    "GitLab" => (
                        "https://gitlab.com/oauth/authorize".to_string(),
                        "https://gitlab.com/oauth/token".to_string(),
                    ),
                    "GitHub" => (
                        "https://github.com/login/oauth/authorize".to_string(),
                        "https://github.com/login/oauth/access_token".to_string(),
                    ),
                    "Google" => (
                        "https://accounts.google.com/o/oauth2/auth".to_string(),
                        "https://oauth2.googleapis.com/token".to_string(),
                    ),
                    _ => (
                        format!("https://example.com/oauth/authorize"),
                        format!("https://example.com/oauth/token"),
                    ),
                };

                // Use provided redirect URI
                let callback_uri = redirect_uri;

                let body = OAuthProviderRequest {
                    provider_type: provider.clone(),
                    client_id,
                    client_secret,
                    auth_url,
                    token_url,
                    scope: scopes_vec,
                    redirect_uri: callback_uri,
                };

                let response = self
                    .make_request_with_app_auth(
                        reqwest::Method::POST,
                        "/mcp/oauth/providers",
                        Some(serde_json::to_value(&body)?),
                        Some("app_20f048e8"), // Use the created app ID
                    )
                    .await?;

                if response.status().is_success() {
                    println!(
                        "{}",
                        format!("{} provider registered successfully!", provider).green()
                    );
                } else {
                    let error_text = response.text().await?;
                    println!(
                        "{}",
                        format!("Failed to register provider: {}", error_text).red()
                    );
                }
            }
            OAuthAction::Authorize {
                provider,
                installation_id,
                redirect_uri,
            } => {
                if redirect_uri.is_some() {
                    println!(
                        "{}",
                        format!(
                            "Starting {} OAuth flow with custom redirect URI...",
                            provider
                        )
                        .blue()
                    );
                } else {
                    println!(
                        "{}",
                        format!(
                            "Starting {} OAuth flow with local callback server...",
                            provider
                        )
                        .blue()
                    );
                }

                // Start local callback server only if using local redirect
                let callback_result = if redirect_uri.is_none() {
                    Some(self.start_oauth_callback_server(3333).await?)
                } else {
                    None
                };

                let body = serde_json::json!({
                    "provider_type": provider,
                    "installation_id": installation_id
                });

                let response = self
                    .make_request_with_app_auth(
                        reqwest::Method::POST,
                        "/mcp/oauth/authorize",
                        Some(body),
                        Some("app_20f048e8"), // Use the created app ID
                    )
                    .await?;

                if response.status().is_success() {
                    // First try to get the raw JSON to see what we received
                    let response_text = response.text().await?;
                    if self.verbose {
                        println!("Response body: {}", response_text);
                    }

                    // Try to parse as JSON
                    match serde_json::from_str::<serde_json::Value>(&response_text) {
                        Ok(json) => {
                            // Check if it has auth_url field
                            if let Some(auth_url) = json.get("auth_url").and_then(|v| v.as_str()) {
                                let state = json
                                    .get("state")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");

                                println!("{}", "Opening browser for authorization...".blue());
                                println!("Authorization URL: {}", auth_url.blue().underline());
                                println!("State: {}", state.yellow());

                                // Try to open browser
                                if let Err(e) = open::that(auth_url) {
                                    println!(
                                        "{}",
                                        format!(
                                            "Failed to open browser: {}. Please open the URL manually.",
                                            e
                                        )
                                        .yellow()
                                    );
                                } else {
                                    println!("{}", "✅ Browser opened successfully!".green());
                                }

                                if let Some(callback_result) = &callback_result {
                                    println!("\n{}", "Waiting for OAuth callback...".blue());

                                    // Wait for callback with timeout
                                    let mut attempts = 0;
                                    let max_attempts = 120; // 2 minutes timeout

                                    while attempts < max_attempts {
                                        if let Some(result) = callback_result.lock().await.clone() {
                                            if let Some(error) = result.error {
                                                println!(
                                                    "{}",
                                                    format!(
                                                        "❌ OAuth authorization failed: {}",
                                                        error
                                                    )
                                                    .red()
                                                );
                                                return Ok(());
                                            }

                                            if let Some(code) = result.code {
                                                println!(
                                                    "{}",
                                                    "✅ Authorization code received!".green()
                                                );

                                                // Automatically process the callback
                                                let callback_body = serde_json::json!({
                                                    "code": code,
                                                    "state": result.state.unwrap_or_default()
                                                });

                                                let callback_response = self
                                                    .make_request_with_app_auth(
                                                        reqwest::Method::POST,
                                                        "/mcp/oauth/callback",
                                                        Some(callback_body),
                                                        Some("app_20f048e8"),
                                                    )
                                                    .await?;

                                                if callback_response.status().is_success() {
                                                    let token_info: Value =
                                                        callback_response.json().await?;

                                                    // Store token info locally
                                                    if let Some(provider_type) = token_info
                                                        .get("provider_type")
                                                        .and_then(|p| p.as_str())
                                                    {
                                                        let oauth_token = OAuthTokenInfo {
                                                            provider_type: provider_type
                                                                .to_string(),
                                                            access_token: token_info
                                                                .get("access_token")
                                                                .and_then(|t| t.as_str())
                                                                .unwrap_or_default()
                                                                .to_string(),
                                                            refresh_token: token_info
                                                                .get("refresh_token")
                                                                .and_then(|t| t.as_str())
                                                                .map(|s| s.to_string()),
                                                            expires_at: None,
                                                            scope: Vec::new(),
                                                            created_at: Utc::now(),
                                                        };

                                                        self.config.oauth_tokens.insert(
                                                            provider.to_string(),
                                                            oauth_token,
                                                        );
                                                        self.save_config()?;
                                                    }

                                                    println!("{}", "🎉 OAuth authorization completed successfully!".green());
                                                    println!(
                                                        "Token response: {}",
                                                        serde_json::to_string_pretty(&token_info)?
                                                    );
                                                } else {
                                                    let error_text =
                                                        callback_response.text().await?;
                                                    println!(
                                                        "{}",
                                                        format!(
                                                            "❌ Token exchange failed: {}",
                                                            error_text
                                                        )
                                                        .red()
                                                    );
                                                }

                                                return Ok(());
                                            }
                                        }

                                        tokio::time::sleep(Duration::from_secs(1)).await;
                                        attempts += 1;

                                        if attempts % 10 == 0 {
                                            println!(
                                                "{}",
                                                format!(
                                                "⏳ Still waiting for authorization... ({}/{}s)",
                                                attempts, max_attempts
                                            )
                                                .yellow()
                                            );
                                        }
                                    }

                                    println!(
                                        "{}",
                                        "⏰ Timeout waiting for OAuth callback. Please try again."
                                            .yellow()
                                    );
                                } else {
                                    println!("\n{}", "After completing authorization, use the 'oauth callback' command with the authorization code.".green());
                                }
                            } else {
                                println!("{}", "OAuth flow started successfully!".green());
                                println!("Response: {}", serde_json::to_string_pretty(&json)?);
                            }
                        }
                        Err(e) => {
                            println!(
                                "{}",
                                "OAuth request succeeded but response format unexpected:".yellow()
                            );
                            println!("Parse error: {}", e);
                            println!("Raw response: {}", response_text);
                        }
                    }
                } else {
                    let error_text = response.text().await?;
                    println!(
                        "{}",
                        format!("Failed to start OAuth flow: {}", error_text).red()
                    );
                }
            }
            OAuthAction::Callback { code, state } => {
                println!("{}", "Processing OAuth callback...".blue());

                let body = serde_json::json!({
                    "code": code,
                    "state": state
                });

                let response = self
                    .make_request_with_app_auth(
                        reqwest::Method::POST,
                        "/mcp/oauth/callback",
                        Some(body),
                        Some("app_20f048e8"), // Use the created app ID
                    )
                    .await?;

                if response.status().is_success() {
                    let token_info: Value = response.json().await?;

                    // Store token info locally
                    if let Some(provider) = token_info.get("provider_type").and_then(|p| p.as_str())
                    {
                        let oauth_token = OAuthTokenInfo {
                            provider_type: provider.to_string(),
                            access_token: token_info
                                .get("access_token")
                                .and_then(|t| t.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            refresh_token: token_info
                                .get("refresh_token")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            expires_at: None,  // Parse from response if available
                            scope: Vec::new(), // Parse from response if available
                            created_at: Utc::now(),
                        };

                        self.config
                            .oauth_tokens
                            .insert(provider.to_string(), oauth_token);
                        self.save_config()?;
                    }

                    println!("{}", "OAuth authorization completed successfully!".green());
                    println!("{}", serde_json::to_string_pretty(&token_info)?);
                } else {
                    let error_text = response.text().await?;
                    println!("{}", format!("OAuth callback failed: {}", error_text).red());
                }
            }
            OAuthAction::List => {
                println!("{}", "OAuth Tokens:".green().bold());

                if self.config.oauth_tokens.is_empty() {
                    println!("  {}", "No OAuth tokens configured".yellow());
                } else {
                    for (provider, token) in &self.config.oauth_tokens {
                        println!("  Provider: {}", provider.blue());
                        println!(
                            "    Created: {}",
                            token.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        if let Some(expires) = token.expires_at {
                            if expires < Utc::now() {
                                println!("    Status: {}", "EXPIRED".red());
                            } else {
                                println!("    Status: {}", "ACTIVE".green());
                            }
                        } else {
                            println!("    Status: {}", "ACTIVE".green());
                        }
                        println!();
                    }
                }
            }
            OAuthAction::Revoke { provider } => {
                println!("{}", format!("Revoking {} OAuth token...", provider).blue());

                // Note: Server doesn't have DELETE /mcp/oauth/tokens endpoint
                // Remove from local config for now
                if self.config.oauth_tokens.remove(&provider).is_some() {
                    self.save_config()?;
                    println!(
                        "{}",
                        format!("{} token removed from local config", provider).green()
                    );
                    println!(
                        "{}",
                        "Note: Server-side revocation not implemented".yellow()
                    );
                } else {
                    println!(
                        "{}",
                        format!("No {} token found in local config", provider).yellow()
                    );
                }
            }
        }
        Ok(())
    }

    /// Handle demo actions with detailed breakpoints
    async fn handle_demo_action(&mut self, action: DemoAction) -> Result<()> {
        match action {
            DemoAction::Full {
                ngrok_url,
                gitlab_client_id,
                gitlab_client_secret,
                auto_confirm,
            } => {
                self.run_full_demo(ngrok_url, gitlab_client_id, gitlab_client_secret, auto_confirm)
                    .await
            }
            DemoAction::GitLab { ngrok_url } => self.run_gitlab_demo(ngrok_url).await,
            DemoAction::SetupOAuth {
                ngrok_url,
                gitlab_client_id,
                gitlab_client_secret,
            } => {
                self.run_oauth_setup_demo(ngrok_url, gitlab_client_id, gitlab_client_secret)
                    .await
            }
        }
    }

    /// Run the complete MCP workflow demo
    async fn run_full_demo(
        &mut self,
        ngrok_url: Option<String>,
        gitlab_client_id: Option<String>,
        gitlab_client_secret: Option<String>,
        auto_confirm: bool,
    ) -> Result<()> {
        println!("{}", "🚀 MCP Complete Workflow Demo".blue().bold());
        println!("{}", "=====================================".blue());
        println!();

        // Step 1: Environment Setup and Validation
        self.demo_breakpoint(
            1,
            "Environment Setup and Validation",
            "We'll first validate that all required environment variables and dependencies are available.",
            auto_confirm,
        ).await?;

        let ngrok_url = self.get_or_prompt_ngrok_url(ngrok_url).await?;
        self.server_url = ngrok_url.clone();
        
        println!("✅ Using NgRok URL: {}", ngrok_url.yellow());
        println!();

        // Step 2: Server Health Check
        self.demo_breakpoint(
            2,
            "Server Health Check",
            "We'll check if the MCP server is running and accessible via the NgRok tunnel.",
            auto_confirm,
        ).await?;

        self.demo_server_health_check().await?;

        // Step 3: MCP App Creation
        self.demo_breakpoint(
            3,
            "MCP App Creation",
            "We'll create a new MCP application that will be used for authentication and API access.",
            auto_confirm,
        ).await?;

        let app_id = self.demo_create_app().await?;

        // Step 4: App Installation
        self.demo_breakpoint(
            4,
            "App Installation with Project Context",
            "We'll install the app with GitLab project context to enable repository-specific operations.",
            auto_confirm,
        ).await?;

        let installation_id = self.demo_install_app(&app_id).await?;

        // Step 5: OAuth Provider Registration
        self.demo_breakpoint(
            5,
            "OAuth Provider Registration",
            "We'll register GitLab as an OAuth provider to enable secure API access.",
            auto_confirm,
        ).await?;

        let (client_id, client_secret) = self.get_or_prompt_gitlab_oauth(gitlab_client_id, gitlab_client_secret).await?;
        self.demo_register_oauth_provider(&app_id, &ngrok_url, &client_id, &client_secret).await?;

        // Step 6: Authentication Token Creation
        self.demo_breakpoint(
            6,
            "Authentication Token Creation",
            "We'll create an installation token for API access.",
            auto_confirm,
        ).await?;

        self.demo_create_auth_token(&app_id, &installation_id).await?;

        // Step 7: OAuth Authorization Flow
        self.demo_breakpoint(
            7,
            "OAuth Authorization Flow",
            "We'll start the OAuth flow and open a browser for GitLab authentication.",
            auto_confirm,
        ).await?;

        self.demo_oauth_flow(&app_id, &installation_id).await?;

        // Step 8: GitLab API Testing
        self.demo_breakpoint(
            8,
            "GitLab API Integration Testing",
            "We'll test the GitLab integration by fetching user information and project data.",
            auto_confirm,
        ).await?;

        self.demo_test_gitlab_integration().await?;

        // Step 9: Project Context Discovery
        self.demo_breakpoint(
            9,
            "Project Context Discovery",
            "We'll discover and display the current project context and available operations.",
            auto_confirm,
        ).await?;

        self.demo_project_context_discovery().await?;

        // Step 10: Issue Management Demo
        self.demo_breakpoint(
            10,
            "Issue Management Demo",
            "We'll demonstrate issue management capabilities by listing and potentially creating issues.",
            auto_confirm,
        ).await?;

        self.demo_issue_management().await?;

        println!();
        println!("{}", "🎉 Demo Complete!".green().bold());
        println!("{}", "The MCP server is now fully configured and ready for use.".green());
        println!();
        println!("{}", "Next Steps:".yellow().bold());
        println!("• Use the MCP tools in your IDE (Cursor, Windsurf, etc.)");
        println!("• Integrate with other GitLab projects");
        println!("• Explore additional MCP capabilities");

        Ok(())
    }

    /// Display a demo breakpoint with explanation
    async fn demo_breakpoint(
        &self,
        step: u32,
        title: &str,
        description: &str,
        auto_confirm: bool,
    ) -> Result<()> {
        println!("{}", format!("📍 Step {}: {}", step, title).cyan().bold());
        println!("{}", format!("   {}", description).white());
        println!();

        if !auto_confirm {
            let proceed = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Ready to proceed?")
                .default(true)
                .interact()?;

            if !proceed {
                println!("{}", "Demo paused. Run again when ready.".yellow());
                std::process::exit(0);
            }
        }

        println!();
        Ok(())
    }

    /// Get or prompt for NgRok URL
    async fn get_or_prompt_ngrok_url(&self, ngrok_url: Option<String>) -> Result<String> {
        match ngrok_url {
            Some(url) => Ok(url),
            None => {
                println!("{}", "NgRok URL Required".yellow().bold());
                println!("Please start your MCP server and create an NgRok tunnel:");
                println!("1. Start server: cargo run --bin server");
                println!("2. Start NgRok: ngrok http 3000");
                println!();

                let url: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter NgRok URL (e.g., https://abc123.ngrok-free.app)")
                    .interact_text()?;

                Ok(url)
            }
        }
    }

    /// Get or prompt for GitLab OAuth credentials
    async fn get_or_prompt_gitlab_oauth(
        &self,
        client_id: Option<String>,
        client_secret: Option<String>,
    ) -> Result<(String, String)> {
        let client_id = match client_id {
            Some(id) => id,
            None => {
                println!("{}", "GitLab OAuth Setup Required".yellow().bold());
                println!("Please create a GitLab OAuth application:");
                println!("1. Go to GitLab → Settings → Applications");
                println!("2. Create new application with these settings:");
                println!("   - Name: MCP Demo Application");
                println!("   - Redirect URI: {}/mcp/oauth/callback", self.server_url);
                println!("   - Scopes: api, read_user, read_repository");
                println!();

                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter GitLab Client ID")
                    .interact_text()?
            }
        };

        let client_secret = match client_secret {
            Some(secret) => secret,
            None => {
                Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter GitLab Client Secret")
                    .interact_text()?
            }
        };

        Ok((client_id, client_secret))
    }

    /// Demo: Server health check
    async fn demo_server_health_check(&self) -> Result<()> {
        println!("{}", "Checking server health...".blue());

        let health_response = self
            .client
            .get(&format!("{}/health", self.server_url))
            .send()
            .await;

        match health_response {
            Ok(response) if response.status().is_success() => {
                println!("✅ Server is healthy and accessible");
                if let Ok(body) = response.text().await {
                    if !body.is_empty() {
                        println!("   Response: {}", body.trim());
                    }
                }
            }
            Ok(response) => {
                println!("⚠️  Server responded with status: {}", response.status());
                if let Ok(body) = response.text().await {
                    println!("   Response: {}", body);
                }
            }
            Err(e) => {
                println!("❌ Server health check failed: {}", e);
                println!("   Please ensure the server is running and NgRok tunnel is active");
                return Err(anyhow::anyhow!("Server health check failed"));
            }
        }

        println!();
        Ok(())
    }

    /// Demo: Create MCP app
    async fn demo_create_app(&mut self) -> Result<String> {
        println!("{}", "Creating MCP application...".blue());

        let app_id = format!("demo_app_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string());
        let (private_key, public_key) = self.generate_test_keys();

        let body = serde_json::json!({
            "app_id": app_id,
            "name": "MCP Demo Application",
            "description": "Demonstration application for MCP GitLab integration",
            "owner": "demo-user",
            "homepage_url": null,
            "webhook_url": null,
            "permissions": {
                "workflows": "Read",
                "agents": "Read",
                "functions": "None",
                "external_apis": "None",
                "webhooks": "None",
                "audit_logs": "None",
                "project_contexts": []
            },
            "events": [],
            "private_key": private_key,
            "public_key": public_key,
            "client_id": format!("client_{}", &app_id[9..]),
            "client_secret": format!("secret_{}", Uuid::new_v4().to_string().replace("-", "")),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339()
        });

        let response = self
            .make_request(reqwest::Method::POST, "/mcp/auth/apps", Some(body))
            .await?;

        if response.status().is_success() {
            println!("✅ App created successfully!");
            println!("   App ID: {}", app_id.yellow());
        } else {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create app: {}", error_text));
        }

        println!();
        Ok(app_id)
    }

    /// Demo: Install app with project context
    async fn demo_install_app(&mut self, app_id: &str) -> Result<String> {
        println!("{}", "Installing app with GitLab project context...".blue());

        // Try to detect current Git repository
        let project_context = self.detect_git_project_context().await;

        let installation_id = format!("demo_inst_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string());

        let body = serde_json::json!({
            "installation_id": installation_id,
            "app_id": app_id,
            "account": {
                "id": "demo_user_123",
                "login": "demo-user",
                "account_type": "User"
            },
            "permissions": {
                "workflows": "Read",
                "agents": "Read",
                "functions": "None",
                "external_apis": "None",
                "webhooks": "None",
                "audit_logs": "None",
                "project_contexts": []
            },
            "project_contexts": project_context,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "suspended_at": null,
            "suspended_by": null
        });

        let response = self
            .make_request(reqwest::Method::POST, "/mcp/auth/installations", Some(body))
            .await?;

        if response.status().is_success() {
            println!("✅ App installed successfully!");
            println!("   Installation ID: {}", installation_id.yellow());
            if !project_context.is_empty() {
                println!("   Project context configured for current repository");
            }
        } else {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to install app: {}", error_text));
        }

        println!();
        Ok(installation_id)
    }

    /// Detect current Git project context
    async fn detect_git_project_context(&self) -> Vec<serde_json::Value> {
        // Try to get git remote URL
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .output()
            .await
        {
            if let Ok(remote_url) = String::from_utf8(output.stdout) {
                let remote_url = remote_url.trim();
                
                // Parse GitLab URL
                if remote_url.contains("gitlab.com") {
                    if let Some(project_path) = self.extract_gitlab_project_path(remote_url) {
                        let parts: Vec<&str> = project_path.split('/').collect();
                        if parts.len() >= 2 {
                            let owner = parts[0];
                            let repo = parts[1];
                            
                            println!("   Detected GitLab project: {}/{}", owner.yellow(), repo.yellow());
                            
                            return vec![serde_json::json!({
                                "context_id": format!("gitlab_{}_{}", owner, repo),
                                "name": format!("{}/{}", owner, repo),
                                "description": format!("GitLab repository: {}/{}", owner, repo),
                                "context_type": {
                                    "GitLab": {
                                        "project_id": format!("{}/{}", owner, repo),
                                        "namespace": owner
                                    }
                                },
                                "configuration": {
                                    "base_url": "https://gitlab.com",
                                    "api_version": "v4",
                                    "default_branch": "main",
                                    "include_patterns": ["**/*"],
                                    "exclude_patterns": [".git/**", "target/**", "node_modules/**"],
                                    "max_depth": 10,
                                    "cache_duration_hours": 24
                                },
                                "metadata": {
                                    "repository_type": "gitlab",
                                    "owner": owner,
                                    "repo": repo,
                                    "full_name": format!("{}/{}", owner, repo)
                                },
                                "created_at": chrono::Utc::now().to_rfc3339(),
                                "last_accessed": null
                            })];
                        }
                    }
                }
            }
        }

        println!("   No GitLab project detected in current directory");
        Vec::new()
    }

    /// Extract GitLab project path from remote URL
    fn extract_gitlab_project_path(&self, remote_url: &str) -> Option<String> {
        // Handle both SSH and HTTPS URLs
        if remote_url.starts_with("git@gitlab.com:") {
            // SSH format: git@gitlab.com:owner/repo.git
            let path = remote_url.strip_prefix("git@gitlab.com:")?;
            let path = path.strip_suffix(".git").unwrap_or(path);
            Some(path.to_string())
        } else if remote_url.starts_with("https://gitlab.com/") {
            // HTTPS format: https://gitlab.com/owner/repo.git
            let path = remote_url.strip_prefix("https://gitlab.com/")?;
            let path = path.strip_suffix(".git").unwrap_or(path);
            Some(path.to_string())
        } else {
            None
        }
    }

    /// Demo: Register OAuth provider
    async fn demo_register_oauth_provider(
        &mut self,
        app_id: &str,
        ngrok_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<()> {
        println!("{}", "Registering GitLab OAuth provider...".blue());

        let redirect_uri = format!("{}/mcp/oauth/callback", ngrok_url);
        
        let body = serde_json::json!({
            "provider_type": "GitLab",
            "client_id": client_id,
            "client_secret": client_secret,
            "auth_url": "https://gitlab.com/oauth/authorize",
            "token_url": "https://gitlab.com/oauth/token",
            "scope": ["api", "read_user", "read_repository"],
            "redirect_uri": redirect_uri
        });

        let response = self
            .make_request_with_app_auth(
                reqwest::Method::POST,
                "/mcp/oauth/providers",
                Some(body),
                Some(app_id),
            )
            .await?;

        if response.status().is_success() {
            println!("✅ GitLab OAuth provider registered successfully!");
            println!("   Redirect URI: {}", redirect_uri.yellow());
        } else {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to register OAuth provider: {}", error_text));
        }

        println!();
        Ok(())
    }

    /// Demo: Create authentication token
    async fn demo_create_auth_token(&mut self, app_id: &str, installation_id: &str) -> Result<()> {
        println!("{}", "Creating installation authentication token...".blue());

        let body = serde_json::json!({
            "permissions": {
                "workflows": "Read",
                "agents": "Read",
                "functions": "None",
                "external_apis": "None",
                "webhooks": "None",
                "audit_logs": "None",
                "project_contexts": []
            }
        });

        let response = self
            .make_request_with_app_auth(
                reqwest::Method::POST,
                &format!("/mcp/auth/installations/{}/tokens", installation_id),
                Some(body),
                Some(app_id),
            )
            .await?;

        if response.status().is_success() {
            let token_response: Value = response.json().await?;

            if let Some(token) = token_response.get("token").and_then(|t| t.as_str()) {
                let session_info = SessionInfo {
                    session_id: format!("demo_session_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
                    jwt_token: token.to_string(),
                    installation_id: Some(installation_id.to_string()),
                    app_id: app_id.to_string(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                    created_at: Utc::now(),
                };

                let session_id = session_info.session_id.clone();
                self.config.sessions.insert(session_id.clone(), session_info);
                self.config.current_session = Some(session_id.clone());
                self.save_config()?;

                println!("✅ Authentication token created successfully!");
                println!("   Session ID: {}", session_id.yellow());
            } else {
                return Err(anyhow::anyhow!("Token response missing token field"));
            }
        } else {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create auth token: {}", error_text));
        }

        println!();
        Ok(())
    }

    /// Demo: OAuth authorization flow
    async fn demo_oauth_flow(&mut self, app_id: &str, installation_id: &str) -> Result<()> {
        println!("{}", "Starting OAuth authorization flow...".blue());

        // Start local callback server
        let callback_result = self.start_oauth_callback_server(3333).await?;

        let body = serde_json::json!({
            "provider_type": "GitLab",
            "installation_id": installation_id
        });

        let response = self
            .make_request_with_app_auth(
                reqwest::Method::POST,
                "/mcp/oauth/authorize",
                Some(body),
                Some(app_id),
            )
            .await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(json) => {
                    if let Some(auth_url) = json.get("auth_url").and_then(|v| v.as_str()) {
                        let state = json.get("state").and_then(|v| v.as_str()).unwrap_or("unknown");

                        println!("✅ OAuth authorization URL generated!");
                        println!("   State: {}", state.yellow());
                        println!();
                        println!("{}", "🌐 Opening browser for GitLab authentication...".blue());
                        println!("   URL: {}", auth_url.blue().underline());

                        // Try to open browser
                        if let Err(e) = open::that(auth_url) {
                            println!("⚠️  Failed to open browser: {}", e);
                            println!("   Please open the URL manually in your browser.");
                        } else {
                            println!("✅ Browser opened successfully!");
                        }

                        println!();
                        println!("{}", "⏳ Waiting for OAuth callback...".blue());
                        println!("   Please complete the authorization in your browser.");

                        // Wait for callback with timeout
                        let mut attempts = 0;
                        let max_attempts = 120; // 2 minutes timeout

                        while attempts < max_attempts {
                            if let Some(result) = callback_result.lock().await.clone() {
                                if let Some(error) = result.error {
                                    return Err(anyhow::anyhow!("OAuth authorization failed: {}", error));
                                }

                                if let Some(code) = result.code {
                                    println!("✅ Authorization code received!");

                                    // Process the callback
                                    let callback_body = serde_json::json!({
                                        "code": code,
                                        "state": result.state.unwrap_or_default()
                                    });

                                    let callback_response = self
                                        .make_request_with_app_auth(
                                            reqwest::Method::POST,
                                            "/mcp/oauth/callback",
                                            Some(callback_body),
                                            Some(app_id),
                                        )
                                        .await?;

                                    if callback_response.status().is_success() {
                                        let token_info: Value = callback_response.json().await?;

                                        // Store token info locally
                                        if let Some(provider_type) = token_info.get("provider_type").and_then(|p| p.as_str()) {
                                            let oauth_token = OAuthTokenInfo {
                                                provider_type: provider_type.to_string(),
                                                access_token: token_info
                                                    .get("access_token")
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or_default()
                                                    .to_string(),
                                                refresh_token: token_info
                                                    .get("refresh_token")
                                                    .and_then(|t| t.as_str())
                                                    .map(|s| s.to_string()),
                                                expires_at: None,
                                                scope: Vec::new(),
                                                created_at: Utc::now(),
                                            };

                                            self.config.oauth_tokens.insert("GitLab".to_string(), oauth_token);
                                            self.save_config()?;
                                        }

                                        println!("✅ OAuth authorization completed successfully!");
                                        println!("   GitLab access token obtained and stored.");
                                        break;
                                    } else {
                                        let error_text = callback_response.text().await?;
                                        return Err(anyhow::anyhow!("Token exchange failed: {}", error_text));
                                    }
                                }
                            }

                            tokio::time::sleep(Duration::from_secs(1)).await;
                            attempts += 1;

                            if attempts % 10 == 0 {
                                println!("   ⏳ Still waiting... ({}/{}s)", attempts, max_attempts);
                            }
                        }

                        if attempts >= max_attempts {
                            return Err(anyhow::anyhow!("Timeout waiting for OAuth callback"));
                        }
                    } else {
                        return Err(anyhow::anyhow!("OAuth response missing auth_url"));
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to parse OAuth response: {}", e));
                }
            }
        } else {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to start OAuth flow: {}", error_text));
        }

        println!();
        Ok(())
    }

    /// Demo: Test GitLab integration
    async fn demo_test_gitlab_integration(&self) -> Result<()> {
        println!("{}", "Testing GitLab API integration...".blue());

        // Test user information
        println!("   Fetching current user information...");
        let user_response = self
            .make_request(reqwest::Method::POST, "/mcp/tools/call", Some(serde_json::json!({
                "name": "gitlab_get_user",
                "arguments": {}
            })))
            .await?;

        if user_response.status().is_success() {
            let user_data: Value = user_response.json().await?;
            if let Some(content) = user_data.get("content").and_then(|c| c.as_array()) {
                if let Some(user_info) = content.first().and_then(|u| u.get("text")) {
                    println!("✅ User information retrieved successfully!");
                    if let Ok(user_json) = serde_json::from_str::<Value>(user_info.as_str().unwrap_or("{}")) {
                        if let Some(username) = user_json.get("username") {
                            println!("   Username: {}", username.as_str().unwrap_or("unknown").yellow());
                        }
                        if let Some(name) = user_json.get("name") {
                            println!("   Name: {}", name.as_str().unwrap_or("unknown").yellow());
                        }
                    }
                }
            }
        } else {
            println!("⚠️  Failed to fetch user information");
        }

        println!();
        Ok(())
    }

    /// Demo: Project context discovery
    async fn demo_project_context_discovery(&self) -> Result<()> {
        println!("{}", "Discovering project context...".blue());

        // List projects
        println!("   Fetching accessible projects...");
        let projects_response = self
            .make_request(reqwest::Method::POST, "/mcp/tools/call", Some(serde_json::json!({
                "name": "gitlab_list_projects",
                "arguments": {
                    "per_page": 5
                }
            })))
            .await?;

        if projects_response.status().is_success() {
            let projects_data: Value = projects_response.json().await?;
            if let Some(content) = projects_data.get("content").and_then(|c| c.as_array()) {
                if let Some(projects_info) = content.first().and_then(|p| p.get("text")) {
                    println!("✅ Projects retrieved successfully!");
                    if let Ok(projects_json) = serde_json::from_str::<Value>(projects_info.as_str().unwrap_or("[]")) {
                        if let Some(projects) = projects_json.as_array() {
                            println!("   Found {} accessible projects:", projects.len());
                            for (i, project) in projects.iter().take(3).enumerate() {
                                if let Some(name) = project.get("name") {
                                    println!("   {}. {}", i + 1, name.as_str().unwrap_or("unknown").yellow());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("⚠️  Failed to fetch projects");
        }

        println!();
        Ok(())
    }

    /// Demo: Issue management
    async fn demo_issue_management(&self) -> Result<()> {
        println!("{}", "Demonstrating issue management...".blue());

        // Try to get current project ID from git context
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .output()
            .await
        {
            if let Ok(remote_url) = String::from_utf8(output.stdout) {
                let remote_url = remote_url.trim();
                if let Some(project_path) = self.extract_gitlab_project_path(remote_url) {
                    println!("   Fetching issues for project: {}", project_path.yellow());
                    
                    let issues_response = self
                        .make_request(reqwest::Method::POST, "/mcp/tools/call", Some(serde_json::json!({
                            "name": "gitlab_list_issues",
                            "arguments": {
                                "project_id": project_path,
                                "state": "opened",
                                "per_page": 5
                            }
                        })))
                        .await?;

                    if issues_response.status().is_success() {
                        let issues_data: Value = issues_response.json().await?;
                        if let Some(content) = issues_data.get("content").and_then(|c| c.as_array()) {
                            if let Some(issues_info) = content.first().and_then(|i| i.get("text")) {
                                println!("✅ Issues retrieved successfully!");
                                if let Ok(issues_json) = serde_json::from_str::<Value>(issues_info.as_str().unwrap_or("[]")) {
                                    if let Some(issues) = issues_json.as_array() {
                                        println!("   Found {} open issues:", issues.len());
                                        for (i, issue) in issues.iter().take(3).enumerate() {
                                            if let Some(title) = issue.get("title") {
                                                println!("   {}. {}", i + 1, title.as_str().unwrap_or("unknown").yellow());
                                            }
                                        }
                                    } else {
                                        println!("   No open issues found.");
                                    }
                                }
                            }
                        }
                    } else {
                        println!("⚠️  Failed to fetch issues");
                    }
                } else {
                    println!("   No GitLab project detected in current directory");
                }
            }
        }

        println!();
        Ok(())
    }

    /// Run GitLab integration demo only
    async fn run_gitlab_demo(&mut self, ngrok_url: Option<String>) -> Result<()> {
        println!("{}", "🔗 GitLab Integration Demo".blue().bold());
        println!("{}", "============================".blue());
        println!();

        let ngrok_url = self.get_or_prompt_ngrok_url(ngrok_url).await?;
        self.server_url = ngrok_url;

        self.demo_test_gitlab_integration().await?;
        self.demo_project_context_discovery().await?;
        self.demo_issue_management().await?;

        println!("{}", "✅ GitLab integration demo complete!".green());
        Ok(())
    }

    /// Run OAuth setup demo only
    async fn run_oauth_setup_demo(
        &mut self,
        ngrok_url: Option<String>,
        gitlab_client_id: Option<String>,
        gitlab_client_secret: Option<String>,
    ) -> Result<()> {
        println!("{}", "🔐 OAuth Setup Demo".blue().bold());
        println!("{}", "===================".blue());
        println!();

        let ngrok_url = self.get_or_prompt_ngrok_url(ngrok_url).await?;
        self.server_url = ngrok_url.clone();

        let (client_id, client_secret) = self.get_or_prompt_gitlab_oauth(gitlab_client_id, gitlab_client_secret).await?;
        
        // For demo purposes, use a dummy app_id
        let app_id = "demo_app_oauth";
        
        self.demo_register_oauth_provider(&app_id, &ngrok_url, &client_id, &client_secret).await?;

        println!("{}", "✅ OAuth setup demo complete!".green());
        Ok(())
    }

    /// Handle session actions
    async fn handle_session_action(&mut self, action: SessionAction) -> Result<()> {
        match action {
            SessionAction::List => {
                println!("{}", "Active Sessions:".green().bold());

                if self.config.sessions.is_empty() {
                    println!("  {}", "No active sessions".yellow());
                } else {
                    for (session_id, session) in &self.config.sessions {
                        let is_current = self.config.current_session.as_ref() == Some(session_id);
                        let marker = if is_current { "→ " } else { "  " };

                        println!("{}Session ID: {}", marker, session_id.yellow());
                        println!("{}  App ID: {}", marker, session.app_id);
                        if let Some(inst_id) = &session.installation_id {
                            println!("{}  Installation ID: {}", marker, inst_id);
                        }
                        println!(
                            "{}  Created: {}",
                            marker,
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );

                        if session.expires_at < Utc::now() {
                            println!("{}  Status: {}", marker, "EXPIRED".red());
                        } else {
                            println!("{}  Status: {}", marker, "ACTIVE".green());
                        }

                        if is_current {
                            println!("{}  {}", marker, "(current)".blue().bold());
                        }
                        println!();
                    }
                }
            }
            SessionAction::Switch { session_id } => {
                if self.config.sessions.contains_key(&session_id) {
                    self.config.current_session = Some(session_id.clone());
                    self.save_config()?;
                    println!("{}", format!("Switched to session: {}", session_id).green());
                } else {
                    println!("{}", format!("Session '{}' not found", session_id).red());
                }
            }
            SessionAction::Current => {
                if let Some(session_id) = &self.config.current_session {
                    if let Some(session) = self.config.sessions.get(session_id) {
                        println!("{}", "Current Session:".green().bold());
                        println!("  Session ID: {}", session.session_id.yellow());
                        println!("  App ID: {}", session.app_id);
                        if let Some(inst_id) = &session.installation_id {
                            println!("  Installation ID: {}", inst_id);
                        }
                        println!(
                            "  Created: {}",
                            session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                        println!(
                            "  Expires: {}",
                            session.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
                        );

                        if session.expires_at < Utc::now() {
                            println!("  Status: {}", "EXPIRED".red().bold());
                        } else {
                            println!("  Status: {}", "ACTIVE".green().bold());
                        }
                    }
                } else {
                    println!("{}", "No current session".yellow());
                }
            }
            SessionAction::Clear => {
                let confirm = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Are you sure you want to clear all sessions?")
                    .default(false)
                    .interact()?;

                if confirm {
                    self.config.sessions.clear();
                    self.config.current_session = None;
                    self.save_config()?;
                    println!("{}", "All sessions cleared!".green());
                } else {
                    println!("{}", "Operation cancelled".yellow());
                }
            }
        }
        Ok(())
    }

    /// Run interactive mode
    async fn run_interactive(&mut self) -> Result<()> {
        println!("{}", "MCP CLI Interactive Mode".blue().bold());
        println!("Use arrow keys to navigate, Enter to select, Ctrl+C to exit");

        loop {
            let options = vec![
                "Server Status",
                "Authentication",
                "OAuth Management",
                "Session Management",
                "Exit",
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select an action")
                .items(&options)
                .default(0)
                .interact()?;

            match selection {
                0 => self.handle_server_action(ServerAction::Status).await?,
                1 => self.interactive_auth().await?,
                2 => self.interactive_oauth().await?,
                3 => self.interactive_session().await?,
                4 => {
                    println!("{}", "Goodbye!".green());
                    break;
                }
                _ => unreachable!(),
            }

            println!(); // Add spacing between operations
        }

        Ok(())
    }

    /// Interactive authentication menu
    async fn interactive_auth(&mut self) -> Result<()> {
        let options = vec![
            "Show Auth Status",
            "Create App",
            "Install App",
            "Login",
            "Logout",
            "List Apps",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Authentication Actions")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.handle_auth_action(AuthAction::Status).await?,
            1 => {
                let name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("App name")
                    .interact_text()?;
                let description: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("App description (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let desc = if description.is_empty() {
                    None
                } else {
                    Some(description)
                };
                self.handle_auth_action(AuthAction::CreateApp {
                    name,
                    description: desc,
                })
                .await?;
            }
            2 => {
                let app_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("App ID")
                    .interact_text()?;
                let context: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Context (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let ctx = if context.is_empty() {
                    None
                } else {
                    Some(context)
                };
                self.handle_auth_action(AuthAction::Install {
                    app_id,
                    context: ctx,
                })
                .await?;
            }
            3 => {
                let app_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("App ID")
                    .interact_text()?;
                let installation_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Installation ID (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let inst_id = if installation_id.is_empty() {
                    None
                } else {
                    Some(installation_id)
                };
                self.handle_auth_action(AuthAction::Login {
                    app_id,
                    installation_id: inst_id,
                })
                .await?;
            }
            4 => self.handle_auth_action(AuthAction::Logout).await?,
            5 => self.handle_auth_action(AuthAction::ListApps).await?,
            6 => return Ok(()),
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Interactive OAuth menu
    async fn interactive_oauth(&mut self) -> Result<()> {
        let options = vec![
            "List OAuth Tokens",
            "Register Provider",
            "Start Authorization",
            "Complete Callback",
            "Revoke Token",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("OAuth Actions")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.handle_oauth_action(OAuthAction::List).await?,
            1 => {
                let providers = vec!["gitlab", "github", "google"];
                let provider_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select provider type")
                    .items(&providers)
                    .default(0)
                    .interact()?;
                let provider = providers[provider_idx].to_string();

                let client_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Client ID")
                    .interact_text()?;
                let client_secret: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Client Secret")
                    .interact_text()?;
                let redirect_uri: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Redirect URI (default: http://localhost:3000/callback)")
                    .default("http://localhost:3000/callback".to_string())
                    .interact_text()?;
                let scopes: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Scopes (comma-separated, optional)")
                    .allow_empty(true)
                    .interact_text()?;
                let scopes_opt = if scopes.is_empty() {
                    None
                } else {
                    Some(scopes)
                };

                self.handle_oauth_action(OAuthAction::Register {
                    provider,
                    client_id,
                    client_secret,
                    redirect_uri,
                    scopes: scopes_opt,
                })
                .await?;
            }
            2 => {
                let providers = vec!["gitlab", "github", "google"];
                let provider_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select provider type")
                    .items(&providers)
                    .default(0)
                    .interact()?;
                let provider = providers[provider_idx].to_string();

                let installation_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Installation ID")
                    .interact_text()?;

                self.handle_oauth_action(OAuthAction::Authorize {
                    provider,
                    installation_id,
                    redirect_uri: None,
                })
                .await?;
            }
            3 => {
                let code: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Authorization Code")
                    .interact_text()?;
                let state: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("State Parameter")
                    .interact_text()?;

                self.handle_oauth_action(OAuthAction::Callback { code, state })
                    .await?;
            }
            4 => {
                let providers = vec!["gitlab", "github", "google"];
                let provider_idx = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select provider to revoke")
                    .items(&providers)
                    .default(0)
                    .interact()?;
                let provider = providers[provider_idx].to_string();

                self.handle_oauth_action(OAuthAction::Revoke { provider })
                    .await?;
            }
            5 => return Ok(()),
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Interactive session menu
    async fn interactive_session(&mut self) -> Result<()> {
        let options = vec![
            "List Sessions",
            "Show Current Session",
            "Switch Session",
            "Clear All Sessions",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Session Actions")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => self.handle_session_action(SessionAction::List).await?,
            1 => self.handle_session_action(SessionAction::Current).await?,
            2 => {
                // First show available sessions
                println!("{}", "Available Sessions:".blue());
                for (session_id, session) in &self.config.sessions {
                    println!("  {} (App: {})", session_id.yellow(), session.app_id);
                }

                let session_id: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Session ID to switch to")
                    .interact_text()?;

                self.handle_session_action(SessionAction::Switch { session_id })
                    .await?;
            }
            3 => self.handle_session_action(SessionAction::Clear).await?,
            4 => return Ok(()),
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Show a loading spinner
    async fn show_loading(&self, message: &str, duration_ms: u64) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈", "⠁"])
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message(message.to_string());

        pb.enable_steady_tick(Duration::from_millis(120));
        sleep(Duration::from_millis(duration_ms)).await;
        pb.finish_with_message(format!("{} ✓", message));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Create CLI application
    let app = CliApp::new(cli.server_url, cli.config, cli.verbose)
        .context("Failed to initialize CLI application")?;

    // Run the application
    if let Err(e) = app.run(cli.command).await {
        eprintln!("{}", format!("Error: {}", e).red());
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_config_serialization() {
        let config = CliConfig {
            server_url: Some("http://localhost:3000".to_string()),
            current_session: Some("session-123".to_string()),
            sessions: HashMap::new(),
            oauth_tokens: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CliConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.server_url, deserialized.server_url);
        assert_eq!(config.current_session, deserialized.current_session);
    }

    #[test]
    fn test_session_info_creation() {
        let session = SessionInfo {
            session_id: "test-session".to_string(),
            jwt_token: "test-token".to_string(),
            installation_id: Some("install-123".to_string()),
            app_id: "app-456".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            created_at: Utc::now(),
        };

        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.app_id, "app-456");
        assert!(session.installation_id.is_some());
    }

    #[tokio::test]
    async fn test_cli_app_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test-config.json");

        let app = CliApp::new(
            "http://localhost:3000".to_string(),
            config_path.to_string_lossy().to_string(),
            false,
        )
        .unwrap();

        assert_eq!(app.server_url, "http://localhost:3000");
        assert!(!app.verbose);
    }
}
