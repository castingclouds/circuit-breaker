// OAuth provider system for external API integration
// This module implements OAuth2 flows for connecting to external APIs like GitLab, GitHub, etc.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use url::Url;

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub provider_type: OAuthProviderType,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub scope: Vec<String>,
    pub redirect_uri: String,
}

/// Supported OAuth provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OAuthProviderType {
    GitLab,
    GitHub,
    Google,
    Custom(String),
}

/// OAuth token response from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Stored OAuth token with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOAuthToken {
    pub provider_type: OAuthProviderType,
    pub installation_id: String,
    pub user_id: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_refreshed: Option<DateTime<Utc>>,
}

/// OAuth authorization request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthRequest {
    pub provider_type: OAuthProviderType,
    pub installation_id: String,
    pub user_id: Option<String>,
    pub state: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
}

/// OAuth callback parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// OAuth manager for handling multiple providers and tokens
pub struct OAuthManager {
    providers: Arc<RwLock<HashMap<OAuthProviderType, OAuthProvider>>>,
    tokens: Arc<RwLock<HashMap<String, StoredOAuthToken>>>,
    pending_auths: Arc<RwLock<HashMap<String, OAuthAuthRequest>>>,
    http_client: Client,
}

impl OAuthManager {
    /// Create a new OAuth manager
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pending_auths: Arc::new(RwLock::new(HashMap::new())),
            http_client: Client::new(),
        }
    }

    /// Register an OAuth provider
    pub async fn register_provider(&self, provider: OAuthProvider) -> Result<()> {
        let provider_type = provider.provider_type.clone();
        let mut providers = self.providers.write().await;
        providers.insert(provider_type.clone(), provider);

        info!("Registered OAuth provider: {:?}", provider_type);
        Ok(())
    }

    /// Get authorization URL for a provider
    pub async fn get_authorization_url(
        &self,
        provider_type: OAuthProviderType,
        installation_id: String,
        user_id: Option<String>,
        redirect_uri: Option<String>,
        scope: Option<Vec<String>>,
    ) -> Result<String> {
        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(&provider_type)
                .ok_or_else(|| anyhow!("Provider {:?} not found", provider_type))?
                .clone()
        };

        // Generate state parameter for CSRF protection
        let state = uuid::Uuid::new_v4().to_string();

        // Use provided redirect URI or default from provider
        let redirect_uri = redirect_uri.unwrap_or(provider.redirect_uri.clone());

        // Use provided scope or default from provider
        let scope = scope.unwrap_or(provider.scope.clone());

        // Store pending auth request
        let auth_request = OAuthAuthRequest {
            provider_type: provider_type.clone(),
            installation_id: installation_id.clone(),
            user_id,
            state: state.clone(),
            redirect_uri: redirect_uri.clone(),
            scope: scope.clone(),
        };

        {
            let mut pending = self.pending_auths.write().await;
            pending.insert(state.clone(), auth_request);
        }

        // Build authorization URL
        let mut url = Url::parse(&provider.auth_url)?;
        url.query_pairs_mut()
            .append_pair("client_id", &provider.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("state", &state)
            .append_pair("scope", &scope.join(" "));

        info!(
            "Generated OAuth authorization URL for {:?}, installation: {}",
            provider_type, installation_id
        );

        Ok(url.to_string())
    }

    /// Handle OAuth callback and exchange code for token
    pub async fn handle_callback(&self, callback: OAuthCallback) -> Result<StoredOAuthToken> {
        // Verify state parameter
        let auth_request = {
            let mut pending = self.pending_auths.write().await;
            pending
                .remove(&callback.state)
                .ok_or_else(|| anyhow!("Invalid or expired state parameter"))?
        };

        // Check for OAuth errors
        if let Some(error) = callback.error {
            return Err(anyhow!(
                "OAuth error: {} - {}",
                error,
                callback.error_description.unwrap_or_default()
            ));
        }

        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(&auth_request.provider_type)
                .ok_or_else(|| anyhow!("Provider {:?} not found", auth_request.provider_type))?
                .clone()
        };

        // Exchange code for token
        let token_response = self
            .exchange_code_for_token(&provider, &callback.code, &auth_request.redirect_uri)
            .await?;

        // Calculate expiration
        let expires_at = token_response
            .expires_in
            .map(|expires_in| Utc::now() + Duration::seconds(expires_in as i64));

        // Create stored token
        let stored_token = StoredOAuthToken {
            provider_type: auth_request.provider_type.clone(),
            installation_id: auth_request.installation_id.clone(),
            user_id: auth_request.user_id,
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
            scope: token_response
                .scope
                .map(|s| s.split(' ').map(|s| s.to_string()).collect())
                .unwrap_or(auth_request.scope),
            created_at: Utc::now(),
            last_refreshed: None,
        };

        // Store token
        let token_key = self.generate_token_key(&stored_token);
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token_key.clone(), stored_token.clone());
        }

        info!(
            "Successfully stored OAuth token for {:?}, installation: {}",
            auth_request.provider_type, auth_request.installation_id
        );

        Ok(stored_token)
    }

    /// Get stored token for a provider and installation
    pub async fn get_token(
        &self,
        provider_type: &OAuthProviderType,
        installation_id: &str,
        user_id: Option<&str>,
    ) -> Result<StoredOAuthToken> {
        let token_key = format!(
            "{:?}:{}:{}",
            provider_type,
            installation_id,
            user_id.unwrap_or("system")
        );

        let mut token = {
            let tokens = self.tokens.read().await;
            tokens
                .get(&token_key)
                .ok_or_else(|| {
                    anyhow!(
                        "Token not found for {:?}:{}",
                        provider_type,
                        installation_id
                    )
                })?
                .clone()
        };

        // Check if token needs refresh
        if self.token_needs_refresh(&token) {
            token = self.refresh_token(token).await?;
        }

        Ok(token)
    }

    /// Refresh an OAuth token
    pub async fn refresh_token(&self, mut token: StoredOAuthToken) -> Result<StoredOAuthToken> {
        let refresh_token = token
            .refresh_token
            .as_ref()
            .ok_or_else(|| anyhow!("No refresh token available"))?;

        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(&token.provider_type)
                .ok_or_else(|| anyhow!("Provider {:?} not found", token.provider_type))?
                .clone()
        };

        debug!("Refreshing OAuth token for {:?}", token.provider_type);

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &provider.client_id),
            ("client_secret", &provider.client_secret),
        ];

        let response = self
            .http_client
            .post(&provider.token_url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Token refresh failed: {}", error_text));
        }

        let token_response: OAuthTokenResponse = response.json().await?;

        // Update token
        token.access_token = token_response.access_token;
        if let Some(new_refresh_token) = token_response.refresh_token {
            token.refresh_token = Some(new_refresh_token);
        }
        token.expires_at = token_response
            .expires_in
            .map(|expires_in| Utc::now() + Duration::seconds(expires_in as i64));
        token.last_refreshed = Some(Utc::now());

        // Store updated token
        let token_key = self.generate_token_key(&token);
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token_key, token.clone());
        }

        info!(
            "Successfully refreshed OAuth token for {:?}",
            token.provider_type
        );

        Ok(token)
    }

    /// Revoke an OAuth token
    pub async fn revoke_token(
        &self,
        provider_type: &OAuthProviderType,
        installation_id: &str,
        user_id: Option<&str>,
    ) -> Result<()> {
        let token_key = format!(
            "{:?}:{}:{}",
            provider_type,
            installation_id,
            user_id.unwrap_or("system")
        );

        let token = {
            let mut tokens = self.tokens.write().await;
            tokens.remove(&token_key)
        };

        if let Some(token) = token {
            // Try to revoke token with provider if they support it
            if let Err(e) = self.revoke_token_with_provider(&token).await {
                warn!("Failed to revoke token with provider: {}", e);
            }

            info!(
                "Revoked OAuth token for {:?}:{}",
                provider_type, installation_id
            );
        }

        Ok(())
    }

    /// Make authenticated API request using stored OAuth token
    pub async fn make_authenticated_request(
        &self,
        provider_type: &OAuthProviderType,
        installation_id: &str,
        user_id: Option<&str>,
        method: Method,
        url: &str,
        body: Option<String>,
        headers: Option<HashMap<String, String>>,
    ) -> Result<reqwest::Response> {
        let token = self
            .get_token(provider_type, installation_id, user_id)
            .await?;

        let mut request = self.http_client.request(method, url);

        // Add OAuth token to Authorization header
        request = request.header("Authorization", format!("Bearer {}", token.access_token));

        // Add additional headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add body if provided
        if let Some(body) = body {
            request = request.body(body);
        }

        let response = request.send().await?;

        if response.status().as_u16() == 401 {
            warn!(
                "Received 401, token may have expired for {:?}",
                provider_type
            );
        }

        Ok(response)
    }

    /// Exchange authorization code for access token
    async fn exchange_code_for_token(
        &self,
        provider: &OAuthProvider,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &provider.client_id),
            ("client_secret", &provider.client_secret),
        ];

        debug!(
            "Exchanging OAuth code for token with {:?}",
            provider.provider_type
        );

        let response = self
            .http_client
            .post(&provider.token_url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Token exchange failed: {}", error_text));
        }

        let token_response: OAuthTokenResponse = response.json().await?;
        Ok(token_response)
    }

    /// Check if token needs refresh
    fn token_needs_refresh(&self, token: &StoredOAuthToken) -> bool {
        if let Some(expires_at) = token.expires_at {
            // Refresh if token expires in the next 5 minutes
            let refresh_threshold = Utc::now() + Duration::minutes(5);
            expires_at <= refresh_threshold
        } else {
            false
        }
    }

    /// Generate a key for storing tokens
    fn generate_token_key(&self, token: &StoredOAuthToken) -> String {
        format!(
            "{:?}:{}:{}",
            token.provider_type,
            token.installation_id,
            token.user_id.as_deref().unwrap_or("system")
        )
    }

    /// Attempt to revoke token with provider
    async fn revoke_token_with_provider(&self, token: &StoredOAuthToken) -> Result<()> {
        let provider = {
            let providers = self.providers.read().await;
            providers
                .get(&token.provider_type)
                .ok_or_else(|| anyhow!("Provider {:?} not found", token.provider_type))?
                .clone()
        };

        // Only some providers support token revocation
        match token.provider_type {
            OAuthProviderType::GitLab => {
                // GitLab supports token revocation
                let revoke_url = provider.token_url.replace("/oauth/token", "/oauth/revoke");
                let params = [
                    ("token", &token.access_token),
                    ("client_id", &provider.client_id),
                    ("client_secret", &provider.client_secret),
                ];

                let response = self
                    .http_client
                    .post(&revoke_url)
                    .form(&params)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let error_text = response.text().await?;
                    return Err(anyhow!("Token revocation failed: {}", error_text));
                }
            }
            OAuthProviderType::GitHub => {
                // GitHub supports token revocation via API
                let revoke_url = "https://api.github.com/applications/{client_id}/grant";
                let url = revoke_url.replace("{client_id}", &provider.client_id);

                let body = serde_json::json!({
                    "access_token": token.access_token
                });

                let response = self
                    .http_client
                    .delete(&url)
                    .basic_auth(&provider.client_id, Some(&provider.client_secret))
                    .json(&body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let error_text = response.text().await?;
                    return Err(anyhow!("Token revocation failed: {}", error_text));
                }
            }
            _ => {
                // Provider doesn't support revocation or we don't know how
                return Ok(());
            }
        }

        Ok(())
    }
}

/// Default OAuth provider configurations
impl OAuthManager {
    /// Create GitLab OAuth provider configuration
    pub fn create_gitlab_provider(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        gitlab_url: Option<String>,
    ) -> OAuthProvider {
        let base_url = gitlab_url.unwrap_or_else(|| "https://gitlab.com".to_string());

        OAuthProvider {
            provider_type: OAuthProviderType::GitLab,
            client_id,
            client_secret,
            auth_url: format!("{}/oauth/authorize", base_url),
            token_url: format!("{}/oauth/token", base_url),
            scope: vec!["read_api".to_string(), "read_repository".to_string()],
            redirect_uri,
        }
    }

    /// Create GitHub OAuth provider configuration
    pub fn create_github_provider(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> OAuthProvider {
        OAuthProvider {
            provider_type: OAuthProviderType::GitHub,
            client_id,
            client_secret,
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scope: vec!["repo".to_string(), "read:user".to_string()],
            redirect_uri,
        }
    }

    /// Create Google OAuth provider configuration
    pub fn create_google_provider(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> OAuthProvider {
        OAuthProvider {
            provider_type: OAuthProviderType::Google,
            client_id,
            client_secret,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scope: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            redirect_uri,
        }
    }
}

impl Default for OAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oauth_manager_creation() {
        let manager = OAuthManager::new();
        assert!(manager.providers.read().await.is_empty());
        assert!(manager.tokens.read().await.is_empty());
    }

    #[test]
    fn test_gitlab_provider_creation() {
        let provider = OAuthManager::create_gitlab_provider(
            "test-client-id".to_string(),
            "test-client-secret".to_string(),
            "http://localhost:3000/callback".to_string(),
            None,
        );

        assert_eq!(provider.provider_type, OAuthProviderType::GitLab);
        assert_eq!(provider.client_id, "test-client-id");
        assert_eq!(provider.auth_url, "https://gitlab.com/oauth/authorize");
        assert!(provider.scope.contains(&"read_api".to_string()));
    }

    #[test]
    fn test_github_provider_creation() {
        let provider = OAuthManager::create_github_provider(
            "test-client-id".to_string(),
            "test-client-secret".to_string(),
            "http://localhost:3000/callback".to_string(),
        );

        assert_eq!(provider.provider_type, OAuthProviderType::GitHub);
        assert_eq!(
            provider.auth_url,
            "https://github.com/login/oauth/authorize"
        );
        assert!(provider.scope.contains(&"repo".to_string()));
    }

    #[tokio::test]
    async fn test_provider_registration() {
        let manager = OAuthManager::new();
        let provider = OAuthManager::create_gitlab_provider(
            "test-id".to_string(),
            "test-secret".to_string(),
            "http://localhost/callback".to_string(),
            None,
        );

        let result = manager.register_provider(provider).await;
        assert!(result.is_ok());

        let providers = manager.providers.read().await;
        assert!(providers.contains_key(&OAuthProviderType::GitLab));
    }

    #[tokio::test]
    async fn test_authorization_url_generation() {
        let manager = OAuthManager::new();
        let provider = OAuthManager::create_gitlab_provider(
            "test-id".to_string(),
            "test-secret".to_string(),
            "http://localhost/callback".to_string(),
            None,
        );

        manager.register_provider(provider).await.unwrap();

        let auth_url = manager
            .get_authorization_url(
                OAuthProviderType::GitLab,
                "test-installation".to_string(),
                Some("test-user".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(auth_url.contains("https://gitlab.com/oauth/authorize"));
        assert!(auth_url.contains("client_id=test-id"));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("state="));
    }

    #[test]
    fn test_token_key_generation() {
        let manager = OAuthManager::new();
        let token = StoredOAuthToken {
            provider_type: OAuthProviderType::GitLab,
            installation_id: "install-123".to_string(),
            user_id: Some("user-456".to_string()),
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: None,
            scope: vec![],
            created_at: Utc::now(),
            last_refreshed: None,
        };

        let key = manager.generate_token_key(&token);
        assert_eq!(key, "GitLab:install-123:user-456");
    }

    #[test]
    fn test_token_needs_refresh() {
        let manager = OAuthManager::new();

        // Token that expires in 2 minutes (should need refresh)
        let mut token = StoredOAuthToken {
            provider_type: OAuthProviderType::GitLab,
            installation_id: "test".to_string(),
            user_id: None,
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() + Duration::minutes(2)),
            scope: vec![],
            created_at: Utc::now(),
            last_refreshed: None,
        };

        assert!(manager.token_needs_refresh(&token));

        // Token that expires in 10 minutes (should not need refresh)
        token.expires_at = Some(Utc::now() + Duration::minutes(10));
        assert!(!manager.token_needs_refresh(&token));

        // Token with no expiration (should not need refresh)
        token.expires_at = None;
        assert!(!manager.token_needs_refresh(&token));
    }
}
