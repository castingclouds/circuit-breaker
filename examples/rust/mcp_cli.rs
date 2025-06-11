//! MCP CLI - Command Line Interface for Multi-Context Protocol Server
//!
//! This CLI provides a comprehensive interface for interacting with the MCP server,
//! including authentication, OAuth provider management, and server operations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

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
    redirect_uri: String,
    scopes: Vec<String>,
}

/// OAuth authorization response
#[derive(Debug, Deserialize)]
struct OAuthAuthResponse {
    auth_url: String,
    state: String,
}

/// MCP CLI Application
#[derive(Parser)]
#[command(name = "mcp-cli")]
#[command(about = "CLI for Multi-Context Protocol Server")]
#[command(version = "0.1.0")]
struct Cli {
    /// Server URL (can also be set via MCP_SERVER_URL env var)
    #[arg(long, env = "MCP_SERVER_URL", default_value = "http://localhost:3000")]
    server_url: String,

    /// Configuration file path
    #[arg(long, default_value = "~/.mcp-cli.json")]
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
        let url = format!("{}{}", self.server_url, path);
        let mut request = self.client.request(method, &url);

        if let Some(token) = self.get_current_token() {
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

    /// Run the CLI application
    async fn run(mut self, command: Commands) -> Result<()> {
        match command {
            Commands::Server { action } => self.handle_server_action(action).await,
            Commands::Auth { action } => self.handle_auth_action(action).await,
            Commands::OAuth { action } => self.handle_oauth_action(action).await,
            Commands::Session { action } => self.handle_session_action(action).await,
            Commands::Interactive => self.run_interactive().await,
        }
    }

    /// Handle server actions
    async fn handle_server_action(&mut self, action: ServerAction) -> Result<()> {
        match action {
            ServerAction::Status => {
                println!("{}", "Checking server status...".blue());

                let response = self
                    .make_request(reqwest::Method::GET, "/api/status", None)
                    .await?;

                if response.status().is_success() {
                    let status: ServerStatus = response
                        .json()
                        .await
                        .context("Failed to parse server status")?;

                    println!("{}", "Server Status:".green().bold());
                    println!("  Status: {}", status.status.green());
                    println!("  Version: {}", status.version);
                    println!("  Uptime: {}", status.uptime);
                    println!("  Active Sessions: {}", status.active_sessions);
                    println!("  Registered Apps: {}", status.registered_apps);
                } else {
                    println!(
                        "{}",
                        format!("Server unavailable: {}", response.status()).red()
                    );
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

                let mut body = serde_json::json!({
                    "name": name
                });

                if let Some(desc) = description {
                    body["description"] = Value::String(desc);
                }

                let response = self
                    .make_request(reqwest::Method::POST, "/api/auth/apps", Some(body))
                    .await?;

                if response.status().is_success() {
                    let app: Value = response.json().await?;
                    println!("{}", "App created successfully!".green());
                    println!("{}", serde_json::to_string_pretty(&app)?);
                } else {
                    let error_text = response.text().await?;
                    println!("{}", format!("Failed to create app: {}", error_text).red());
                }
            }
            AuthAction::Install { app_id, context } => {
                println!("{}", format!("Installing app '{}'...", app_id).blue());

                let mut body = serde_json::json!({
                    "app_id": app_id
                });

                if let Some(ctx) = context {
                    body["context"] = Value::String(ctx);
                }

                let response = self
                    .make_request(reqwest::Method::POST, "/api/auth/installations", Some(body))
                    .await?;

                if response.status().is_success() {
                    let installation: Value = response.json().await?;
                    println!("{}", "App installed successfully!".green());
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
                println!("{}", "Logging in...".blue());

                let mut body = serde_json::json!({
                    "app_id": app_id
                });

                if let Some(ref inst_id) = installation_id {
                    body["installation_id"] = Value::String(inst_id.clone());
                }

                let response = self
                    .make_request(reqwest::Method::POST, "/api/auth/login", Some(body))
                    .await?;

                if response.status().is_success() {
                    let login_response: Value = response.json().await?;

                    if let (Some(token), Some(session_id)) = (
                        login_response.get("token").and_then(|t| t.as_str()),
                        login_response.get("session_id").and_then(|s| s.as_str()),
                    ) {
                        let session_info = SessionInfo {
                            session_id: session_id.to_string(),
                            jwt_token: token.to_string(),
                            installation_id: installation_id,
                            app_id: app_id.clone(),
                            expires_at: Utc::now() + chrono::Duration::hours(24), // Default expiry
                            created_at: Utc::now(),
                        };

                        self.config
                            .sessions
                            .insert(session_id.to_string(), session_info);
                        self.config.current_session = Some(session_id.to_string());
                        self.save_config()?;

                        println!("{}", "Login successful!".green());
                        println!("Session ID: {}", session_id.yellow());
                    } else {
                        println!("{}", "Login response missing required fields".red());
                    }
                } else {
                    let error_text = response.text().await?;
                    println!("{}", format!("Login failed: {}", error_text).red());
                }
            }
            AuthAction::Logout => {
                if let Some(session_id) = &self.config.current_session {
                    let response = self
                        .make_request(reqwest::Method::POST, "/api/auth/logout", None)
                        .await?;

                    if response.status().is_success() {
                        self.config.sessions.remove(session_id);
                        self.config.current_session = None;
                        self.save_config()?;
                        println!("{}", "Logged out successfully!".green());
                    } else {
                        println!(
                            "{}",
                            "Failed to logout from server, clearing local session".yellow()
                        );
                        self.config.current_session = None;
                        self.save_config()?;
                    }
                } else {
                    println!("{}", "No active session to logout".yellow());
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
                let response = self
                    .make_request(reqwest::Method::GET, "/api/auth/apps", None)
                    .await?;

                if response.status().is_success() {
                    let apps: Value = response.json().await?;
                    println!("{}", "Registered Apps:".green().bold());
                    println!("{}", serde_json::to_string_pretty(&apps)?);
                } else {
                    println!(
                        "{}",
                        format!("Failed to list apps: {}", response.status()).red()
                    );
                }
            }
        }
        Ok(())
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

                let body = OAuthProviderRequest {
                    provider_type: provider.clone(),
                    client_id,
                    client_secret,
                    redirect_uri,
                    scopes: scopes_vec,
                };

                let response = self
                    .make_request(
                        reqwest::Method::POST,
                        "/api/oauth/providers",
                        Some(serde_json::to_value(&body)?),
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
            } => {
                println!("{}", format!("Starting {} OAuth flow...", provider).blue());

                let body = serde_json::json!({
                    "provider_type": provider,
                    "installation_id": installation_id
                });

                let response = self
                    .make_request(reqwest::Method::POST, "/api/oauth/authorize", Some(body))
                    .await?;

                if response.status().is_success() {
                    let auth_response: OAuthAuthResponse = response.json().await?;

                    println!("{}", "Opening browser for authorization...".blue());
                    println!(
                        "Authorization URL: {}",
                        auth_response.auth_url.blue().underline()
                    );
                    println!("State: {}", auth_response.state.yellow());

                    // Try to open browser
                    if let Err(e) = open::that(&auth_response.auth_url) {
                        println!(
                            "{}",
                            format!(
                                "Failed to open browser: {}. Please open the URL manually.",
                                e
                            )
                            .yellow()
                        );
                    }

                    println!("\n{}", "After completing authorization, use the 'callback' command with the authorization code.".green());
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
                    .make_request(reqwest::Method::POST, "/api/oauth/callback", Some(body))
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

                let body = serde_json::json!({
                    "provider_type": provider
                });

                let response = self
                    .make_request(reqwest::Method::DELETE, "/api/oauth/tokens", Some(body))
                    .await?;

                if response.status().is_success() {
                    self.config.oauth_tokens.remove(&provider);
                    self.save_config()?;
                    println!(
                        "{}",
                        format!("{} OAuth token revoked successfully!", provider).green()
                    );
                } else {
                    println!(
                        "{}",
                        "Failed to revoke token from server, removing locally".yellow()
                    );
                    self.config.oauth_tokens.remove(&provider);
                    self.save_config()?;
                }
            }
        }
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
