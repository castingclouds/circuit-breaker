//! OAuth Auto-Setup for Remote MCP Server
//!
//! This module handles automatic OAuth provider setup for the remote MCP server.
//! It reads OAuth configuration from environment variables and automatically
//! registers providers when the server starts.

use crate::api::mcp_server::MCPServerManager;
use crate::api::oauth::{OAuthProvider, OAuthProviderType};
use anyhow::{anyhow, Result};
use std::env;
use tracing::{debug, error, info, warn};

/// OAuth configuration from environment variables
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub enabled: bool,
    pub default_provider: OAuthProviderType,
    pub callback_url: String,
    pub providers: Vec<OAuthProviderConfig>,
}

/// Configuration for a single OAuth provider
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    pub provider_type: OAuthProviderType,
    pub client_id: String,
    pub client_secret: String,
    pub scope: Vec<String>,
    pub enabled: bool,
}

impl OAuthConfig {
    /// Load OAuth configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let enabled = env::var("MCP_OAUTH_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if !enabled {
            return Ok(Self {
                enabled: false,
                default_provider: OAuthProviderType::GitHub,
                callback_url: String::new(),
                providers: Vec::new(),
            });
        }

        let default_provider = env::var("MCP_OAUTH_DEFAULT_PROVIDER")
            .unwrap_or_else(|_| "github".to_string())
            .parse()
            .unwrap_or(OAuthProviderType::GitHub);

        let callback_url = env::var("MCP_OAUTH_CALLBACK_URL")
            .unwrap_or_else(|_| "http://localhost:8080/mcp/remote/oauth/callback".to_string());

        let mut providers = Vec::new();

        // GitHub OAuth provider
        if let (Ok(client_id), Ok(client_secret)) = (
            env::var("GITHUB_OAUTH_CLIENT_ID"),
            env::var("GITHUB_OAUTH_CLIENT_SECRET"),
        ) {
            if client_id != "your_github_client_id_here"
                && client_secret != "your_github_client_secret_here"
            {
                let scope = env::var("GITHUB_OAUTH_SCOPE")
                    .unwrap_or_else(|_| "read:user,repo".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                providers.push(OAuthProviderConfig {
                    provider_type: OAuthProviderType::GitHub,
                    client_id,
                    client_secret,
                    scope,
                    enabled: true,
                });
            }
        }

        // GitLab OAuth provider
        if let (Ok(client_id), Ok(client_secret)) = (
            env::var("GITLAB_OAUTH_CLIENT_ID"),
            env::var("GITLAB_OAUTH_CLIENT_SECRET"),
        ) {
            if client_id != "your_gitlab_client_id_here"
                && client_secret != "your_gitlab_client_secret_here"
            {
                let scope = env::var("GITLAB_OAUTH_SCOPE")
                    .unwrap_or_else(|_| "read_user,api".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                providers.push(OAuthProviderConfig {
                    provider_type: OAuthProviderType::GitLab,
                    client_id,
                    client_secret,
                    scope,
                    enabled: true,
                });
            }
        }

        // Google OAuth provider
        if let (Ok(client_id), Ok(client_secret)) = (
            env::var("GOOGLE_OAUTH_CLIENT_ID"),
            env::var("GOOGLE_OAUTH_CLIENT_SECRET"),
        ) {
            if client_id != "your_google_client_id_here"
                && client_secret != "your_google_client_secret_here"
            {
                let scope = env::var("GOOGLE_OAUTH_SCOPE")
                    .unwrap_or_else(|_| "openid,profile,email".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                providers.push(OAuthProviderConfig {
                    provider_type: OAuthProviderType::Google,
                    client_id,
                    client_secret,
                    scope,
                    enabled: true,
                });
            }
        }

        Ok(Self {
            enabled,
            default_provider,
            callback_url,
            providers,
        })
    }

    /// Check if any OAuth providers are configured
    pub fn has_providers(&self) -> bool {
        !self.providers.is_empty()
    }

    /// Get provider configuration by type
    pub fn get_provider(&self, provider_type: &OAuthProviderType) -> Option<&OAuthProviderConfig> {
        self.providers
            .iter()
            .find(|p| &p.provider_type == provider_type)
    }

    /// Get the default provider configuration
    pub fn get_default_provider(&self) -> Option<&OAuthProviderConfig> {
        self.get_provider(&self.default_provider)
    }
}

impl std::str::FromStr for OAuthProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(OAuthProviderType::GitHub),
            "gitlab" => Ok(OAuthProviderType::GitLab),
            "google" => Ok(OAuthProviderType::Google),
            other => Ok(OAuthProviderType::Custom(other.to_string())),
        }
    }
}

/// Setup OAuth providers for the MCP server
pub async fn setup_oauth_providers(manager: &MCPServerManager) -> Result<OAuthConfig> {
    info!("🔐 Setting up OAuth providers for remote MCP server...");

    let config = OAuthConfig::from_env()?;

    if !config.enabled {
        info!("OAuth is disabled for remote MCP server");
        return Ok(config);
    }

    if !config.has_providers() {
        warn!("OAuth is enabled but no providers are configured");
        warn!("Please set OAuth client credentials in your .env file:");
        warn!("  GITHUB_OAUTH_CLIENT_ID=your_client_id");
        warn!("  GITHUB_OAUTH_CLIENT_SECRET=your_client_secret");
        return Ok(config);
    }

    info!(
        "📋 Configuring {} OAuth provider(s)...",
        config.providers.len()
    );

    for provider_config in &config.providers {
        match setup_single_provider(manager, provider_config, &config.callback_url).await {
            Ok(_) => {
                info!(
                    "✅ {} OAuth provider configured successfully",
                    format_provider_type(&provider_config.provider_type)
                );
            }
            Err(e) => {
                error!(
                    "❌ Failed to configure {} OAuth provider: {}",
                    format_provider_type(&provider_config.provider_type),
                    e
                );
            }
        }
    }

    info!("🔗 OAuth callback URL: {}", config.callback_url);
    info!(
        "🎯 Default OAuth provider: {}",
        format_provider_type(&config.default_provider)
    );

    Ok(config)
}

/// Setup a single OAuth provider
async fn setup_single_provider(
    manager: &MCPServerManager,
    provider_config: &OAuthProviderConfig,
    callback_url: &str,
) -> Result<()> {
    debug!(
        "Setting up OAuth provider: {:?}",
        provider_config.provider_type
    );

    let provider = create_oauth_provider(provider_config, callback_url)?;

    manager
        .register_oauth_provider(provider)
        .await
        .map_err(|e| anyhow!("Failed to register OAuth provider: {}", e))?;

    Ok(())
}

/// Create OAuth provider from configuration
fn create_oauth_provider(
    config: &OAuthProviderConfig,
    callback_url: &str,
) -> Result<OAuthProvider> {
    let (auth_url, token_url) = get_provider_urls(&config.provider_type)?;

    Ok(OAuthProvider {
        provider_type: config.provider_type.clone(),
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        auth_url,
        token_url,
        scope: config.scope.clone(),
        redirect_uri: callback_url.to_string(),
    })
}

/// Get OAuth URLs for different providers
fn get_provider_urls(provider_type: &OAuthProviderType) -> Result<(String, String)> {
    match provider_type {
        OAuthProviderType::GitHub => Ok((
            "https://github.com/login/oauth/authorize".to_string(),
            "https://github.com/login/oauth/access_token".to_string(),
        )),
        OAuthProviderType::GitLab => Ok((
            "https://gitlab.com/oauth/authorize".to_string(),
            "https://gitlab.com/oauth/token".to_string(),
        )),
        OAuthProviderType::Google => Ok((
            "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            "https://oauth2.googleapis.com/token".to_string(),
        )),
        OAuthProviderType::Custom(name) => Err(anyhow!(
            "Custom OAuth provider '{}' requires manual configuration",
            name
        )),
    }
}

/// Format provider type for display
fn format_provider_type(provider_type: &OAuthProviderType) -> String {
    match provider_type {
        OAuthProviderType::GitHub => "GitHub".to_string(),
        OAuthProviderType::GitLab => "GitLab".to_string(),
        OAuthProviderType::Google => "Google".to_string(),
        OAuthProviderType::Custom(name) => name.clone(),
    }
}

/// Print OAuth setup instructions
pub fn print_oauth_setup_instructions() {
    println!();
    println!("🔐 Remote MCP Server OAuth Setup Instructions");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("To enable OAuth authentication for your remote MCP server:");
    println!();
    println!("1. Create OAuth applications:");
    println!();
    println!("   📘 GitHub:");
    println!("   - Go to: https://github.com/settings/developers");
    println!("   - Click 'New OAuth App'");
    println!("   - Authorization callback URL: http://localhost:8080/mcp/remote/oauth/callback");
    println!();
    println!("   🦊 GitLab:");
    println!("   - Go to: https://gitlab.com/-/profile/applications");
    println!("   - Click 'New application'");
    println!("   - Redirect URI: http://localhost:8080/mcp/remote/oauth/callback");
    println!();
    println!("   🔍 Google:");
    println!("   - Go to: https://console.cloud.google.com/apis/credentials");
    println!("   - Create OAuth 2.0 Client ID");
    println!("   - Authorized redirect URI: http://localhost:8080/mcp/remote/oauth/callback");
    println!();
    println!("2. Update your .env file:");
    println!();
    println!("   MCP_OAUTH_ENABLED=true");
    println!("   MCP_OAUTH_DEFAULT_PROVIDER=github");
    println!("   GITHUB_OAUTH_CLIENT_ID=your_client_id_here");
    println!("   GITHUB_OAUTH_CLIENT_SECRET=your_client_secret_here");
    println!();
    println!("3. Restart the server and visit:");
    println!("   http://localhost:8080/mcp/remote");
    println!();
    println!("4. Configure your MCP client:");
    println!();
    println!("   {{");
    println!("     \"mcpServers\": {{");
    println!("       \"circuit-breaker\": {{");
    println!("         \"command\": \"npx\",");
    println!("         \"args\": [");
    println!("           \"mcp-remote\",");
    println!("           \"http://localhost:8080/mcp/remote/sse\"");
    println!("         ]");
    println!("       }}");
    println!("     }}");
    println!("   }}");
    println!();
}

/// Validate OAuth configuration
pub fn validate_oauth_config(config: &OAuthConfig) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    if !config.has_providers() {
        return Err(anyhow!("OAuth is enabled but no providers are configured"));
    }

    // Validate callback URL
    if config.callback_url.is_empty() {
        return Err(anyhow!("OAuth callback URL is not configured"));
    }

    if !config.callback_url.starts_with("http://") && !config.callback_url.starts_with("https://") {
        return Err(anyhow!(
            "OAuth callback URL must start with http:// or https://"
        ));
    }

    // Validate each provider
    for provider in &config.providers {
        if provider.client_id.is_empty() {
            return Err(anyhow!(
                "{} OAuth client ID is empty",
                format_provider_type(&provider.provider_type)
            ));
        }

        if provider.client_secret.is_empty() {
            return Err(anyhow!(
                "{} OAuth client secret is empty",
                format_provider_type(&provider.provider_type)
            ));
        }

        if provider.scope.is_empty() {
            warn!(
                "{} OAuth scope is empty - this may limit functionality",
                format_provider_type(&provider.provider_type)
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_oauth_config_disabled() {
        env::set_var("MCP_OAUTH_ENABLED", "false");
        let config = OAuthConfig::from_env().unwrap();
        assert!(!config.enabled);
        assert!(!config.has_providers());
    }

    #[test]
    fn test_oauth_provider_type_parsing() {
        assert!(matches!(
            "github".parse::<OAuthProviderType>().unwrap(),
            OAuthProviderType::GitHub
        ));
        assert!(matches!(
            "gitlab".parse::<OAuthProviderType>().unwrap(),
            OAuthProviderType::GitLab
        ));
        assert!(matches!(
            "google".parse::<OAuthProviderType>().unwrap(),
            OAuthProviderType::Google
        ));
    }

    #[test]
    fn test_provider_urls() {
        let (auth_url, token_url) = get_provider_urls(&OAuthProviderType::GitHub).unwrap();
        assert!(auth_url.contains("github.com"));
        assert!(token_url.contains("github.com"));
    }

    #[test]
    fn test_config_validation() {
        let config = OAuthConfig {
            enabled: true,
            default_provider: OAuthProviderType::GitHub,
            callback_url: "http://localhost:8080/callback".to_string(),
            providers: vec![OAuthProviderConfig {
                provider_type: OAuthProviderType::GitHub,
                client_id: "test_id".to_string(),
                client_secret: "test_secret".to_string(),
                scope: vec!["read:user".to_string()],
                enabled: true,
            }],
        };

        assert!(validate_oauth_config(&config).is_ok());
    }

    #[test]
    fn test_config_validation_empty_callback() {
        let config = OAuthConfig {
            enabled: true,
            default_provider: OAuthProviderType::GitHub,
            callback_url: "".to_string(),
            providers: vec![],
        };

        assert!(validate_oauth_config(&config).is_err());
    }
}
