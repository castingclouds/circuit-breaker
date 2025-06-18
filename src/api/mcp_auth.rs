// MCP JWT Authentication Service
// Implements GitHub Apps-style authentication for Circuit Breaker MCP servers

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::mcp_types::*;

/// JWT Authentication Service for MCP
pub struct MCPJWTService {
    /// Private keys for different apps (app_id -> private_key)
    app_private_keys: Arc<RwLock<HashMap<String, EncodingKey>>>,
    /// Public keys for verification (app_id -> public_key)
    app_public_keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    /// Installation store for validating installations
    installation_store: Arc<RwLock<HashMap<String, MCPInstallation>>>,
    /// Session token store for tracking active sessions
    session_store: Arc<RwLock<HashMap<String, SessionTokenData>>>,
    /// Revoked tokens (token_id -> revoked_at)
    revoked_tokens: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    /// Algorithm used for JWT signing
    algorithm: Algorithm,
}

/// JWT Claims for MCP App authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPAppClaims {
    /// Issuer - the app_id
    pub iss: String,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// Audience - typically "mcp-server"
    pub aud: String,
}

/// JWT Claims for Installation tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPInstallationClaims {
    /// Issuer - the app_id
    pub iss: String,
    /// Subject - the installation_id
    pub sub: String,
    /// App ID
    pub app_id: String,
    /// Installation ID
    pub installation_id: String,
    /// Permissions granted to this installation
    pub permissions: MCPPermissions,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// JWT ID - unique identifier for this token
    pub jti: String,
    /// Scopes granted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

/// Session token claims for authenticated sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSessionClaims {
    /// Installation ID
    pub installation_id: String,
    /// App ID
    pub app_id: String,
    /// Session ID
    pub session_id: String,
    /// User ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Permissions for this session
    pub permissions: MCPSessionPermissions,
    /// Project contexts available
    pub project_contexts: Vec<String>,
    /// Issued at
    pub iat: i64,
    /// Expiration time
    pub exp: i64,
    /// JWT ID
    pub jti: String,
}

/// Session token data stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenData {
    pub token_id: String,
    pub installation_id: String,
    pub app_id: String,
    pub session_id: String,
    pub permissions: MCPSessionPermissions,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Installation token containing the JWT and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPInstallationToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub permissions: MCPPermissions,
    pub installation_id: String,
    pub app_id: String,
}

/// Token validation result
#[derive(Debug, Clone)]
pub struct MCPTokenClaims {
    pub installation_id: String,
    pub app_id: String,
    pub permissions: MCPPermissions,
    pub token_type: TokenType,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub project_contexts: Vec<String>,
}

/// Type of token being validated
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    App,
    Installation,
    Session,
}

impl MCPJWTService {
    /// Create a new JWT service
    pub fn new() -> Self {
        Self {
            app_private_keys: Arc::new(RwLock::new(HashMap::new())),
            app_public_keys: Arc::new(RwLock::new(HashMap::new())),
            installation_store: Arc::new(RwLock::new(HashMap::new())),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            revoked_tokens: Arc::new(RwLock::new(HashMap::new())),
            algorithm: Algorithm::RS256,
        }
    }

    /// Register an app with its RSA key pair
    pub async fn register_app(&self, app: MCPApp) -> Result<()> {
        let app_id = app.app_id.clone();

        // Parse the private key
        let private_key = EncodingKey::from_rsa_pem(app.private_key.as_bytes())
            .map_err(|e| anyhow!("Failed to parse private key for app {}: {}", app_id, e))?;

        // Parse the public key
        let public_key = DecodingKey::from_rsa_pem(app.public_key.as_bytes())
            .map_err(|e| anyhow!("Failed to parse public key for app {}: {}", app_id, e))?;

        // Store the keys
        {
            let mut private_keys = self.app_private_keys.write().await;
            private_keys.insert(app_id.clone(), private_key);
        }

        {
            let mut public_keys = self.app_public_keys.write().await;
            public_keys.insert(app_id.clone(), public_key);
        }

        info!("Registered MCP app: {}", app_id);
        Ok(())
    }

    /// Register an installation
    pub async fn register_installation(&self, installation: MCPInstallation) -> Result<()> {
        let installation_id = installation.installation_id.clone();
        let mut store = self.installation_store.write().await;
        store.insert(installation_id.clone(), installation);

        info!("Registered MCP installation: {}", installation_id);
        Ok(())
    }

    /// Generate an App JWT token (short-lived, used to create installation tokens)
    pub async fn generate_app_jwt(&self, app_id: &str, audience: &str) -> Result<String> {
        let private_key = {
            let keys = self.app_private_keys.read().await;
            keys.get(app_id)
                .ok_or_else(|| anyhow!("App {} not found", app_id))?
                .clone()
        };

        let now = Utc::now();
        let claims = MCPAppClaims {
            iss: app_id.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(10)).timestamp(), // App JWTs are short-lived
            aud: audience.to_string(),
        };

        let header = Header::new(self.algorithm);
        let token = encode(&header, &claims, &private_key)
            .map_err(|e| anyhow!("Failed to encode app JWT: {}", e))?;

        debug!("Generated app JWT for app: {}", app_id);
        Ok(token)
    }

    /// Create an installation token (longer-lived, used for actual API access)
    pub async fn create_installation_token(
        &self,
        app_id: &str,
        installation_id: &str,
        requested_permissions: Option<MCPPermissions>,
    ) -> Result<MCPInstallationToken> {
        // Verify the installation exists and belongs to this app
        let installation = {
            let store = self.installation_store.read().await;
            store
                .get(installation_id)
                .ok_or_else(|| anyhow!("Installation {} not found", installation_id))?
                .clone()
        };

        if installation.app_id != app_id {
            return Err(anyhow!(
                "Installation {} does not belong to app {}",
                installation_id,
                app_id
            ));
        }

        // Check if installation is suspended
        if installation.suspended_at.is_some() {
            return Err(anyhow!("Installation {} is suspended", installation_id));
        }

        // Determine effective permissions (intersection of requested and granted)
        let effective_permissions = if let Some(requested) = requested_permissions {
            self.intersect_permissions(&installation.permissions, &requested)
        } else {
            installation.permissions.clone()
        };

        let private_key = {
            let keys = self.app_private_keys.read().await;
            keys.get(app_id)
                .ok_or_else(|| anyhow!("App {} not found", app_id))?
                .clone()
        };

        let now = Utc::now();
        let expires_at = now + Duration::hours(1); // Installation tokens last 1 hour
        let token_id = uuid::Uuid::new_v4().to_string();

        let claims = MCPInstallationClaims {
            iss: app_id.to_string(),
            sub: installation_id.to_string(),
            app_id: app_id.to_string(),
            installation_id: installation_id.to_string(),
            permissions: effective_permissions.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: token_id,
            scopes: None, // TODO: Add scope support
        };

        let header = Header::new(self.algorithm);
        let token = encode(&header, &claims, &private_key)
            .map_err(|e| anyhow!("Failed to encode installation token: {}", e))?;

        info!(
            "Created installation token for app: {}, installation: {}",
            app_id, installation_id
        );

        Ok(MCPInstallationToken {
            token,
            expires_at,
            permissions: effective_permissions,
            installation_id: installation_id.to_string(),
            app_id: app_id.to_string(),
        })
    }

    /// Create a session token for a specific user session
    pub async fn create_session_token(
        &self,
        installation_id: &str,
        session_id: &str,
        user_id: Option<String>,
        permissions: MCPSessionPermissions,
        project_contexts: Vec<String>,
        client_info: ClientInfo,
    ) -> Result<String> {
        // Verify the installation exists
        let installation = {
            let store = self.installation_store.read().await;
            store
                .get(installation_id)
                .ok_or_else(|| anyhow!("Installation {} not found", installation_id))?
                .clone()
        };

        let private_key = {
            let keys = self.app_private_keys.read().await;
            keys.get(&installation.app_id)
                .ok_or_else(|| anyhow!("App {} not found", installation.app_id))?
                .clone()
        };

        let now = Utc::now();
        let expires_at = now + Duration::hours(24); // Session tokens last 24 hours
        let token_id = uuid::Uuid::new_v4().to_string();

        let claims = MCPSessionClaims {
            installation_id: installation_id.to_string(),
            app_id: installation.app_id.clone(),
            session_id: session_id.to_string(),
            user_id: user_id.clone(),
            permissions: permissions.clone(),
            project_contexts: project_contexts.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: token_id.clone(),
        };

        let header = Header::new(self.algorithm);
        let token = encode(&header, &claims, &private_key)
            .map_err(|e| anyhow!("Failed to encode session token: {}", e))?;

        // Store session data
        let session_data = SessionTokenData {
            token_id: token_id.clone(),
            installation_id: installation_id.to_string(),
            app_id: installation.app_id.clone(),
            session_id: session_id.to_string(),
            permissions,
            created_at: now,
            expires_at,
            last_used: now,
            ip_address: client_info.ip_address,
            user_agent: client_info.user_agent,
        };

        {
            let mut store = self.session_store.write().await;
            store.insert(token_id.clone(), session_data);
        }

        info!(
            "Created session token for installation: {}, session: {}",
            installation_id, session_id
        );
        Ok(token)
    }

    /// Validate any type of MCP token
    pub async fn validate_token(&self, token: &str) -> Result<MCPTokenClaims> {
        // First, try to decode without verification to get the claims and determine the app
        let _header = jsonwebtoken::decode_header(token)
            .map_err(|e| anyhow!("Failed to decode token header: {}", e))?;

        // Try different claim types to determine token type
        if let Ok(claims) = self.try_decode_as_session_token(token).await {
            self.validate_session_token_claims(claims).await
        } else if let Ok(claims) = self.try_decode_as_installation_token(token).await {
            self.validate_installation_token_claims(claims).await
        } else if let Ok(claims) = self.try_decode_as_app_token(token).await {
            self.validate_app_token_claims(claims).await
        } else {
            Err(anyhow!("Invalid token format"))
        }
    }

    /// Try to decode token as session token
    async fn try_decode_as_session_token(&self, token: &str) -> Result<MCPSessionClaims> {
        // Decode without verification first to get the app_id
        let unverified: jsonwebtoken::TokenData<MCPSessionClaims> = decode(
            token,
            &DecodingKey::from_secret(&[]), // Dummy key for unverified decode
            &Validation::new(self.algorithm),
        )
        .or_else(|_| {
            // Try with no verification
            let mut validation = Validation::new(self.algorithm);
            validation.insecure_disable_signature_validation();
            decode(token, &DecodingKey::from_secret(&[]), &validation)
        })?;

        let app_id = &unverified.claims.app_id;

        // Get the proper public key
        let public_key = {
            let keys = self.app_public_keys.read().await;
            keys.get(app_id)
                .ok_or_else(|| anyhow!("App {} not found", app_id))?
                .clone()
        };

        // Now decode with proper verification
        let verified =
            decode::<MCPSessionClaims>(token, &public_key, &Validation::new(self.algorithm))
                .map_err(|e| anyhow!("Token verification failed: {}", e))?;

        Ok(verified.claims)
    }

    /// Try to decode token as installation token
    async fn try_decode_as_installation_token(&self, token: &str) -> Result<MCPInstallationClaims> {
        // Similar process as session token
        let unverified: jsonwebtoken::TokenData<MCPInstallationClaims> =
            decode(token, &DecodingKey::from_secret(&[]), &{
                let mut validation = Validation::new(self.algorithm);
                validation.insecure_disable_signature_validation();
                validation
            })?;

        let app_id = &unverified.claims.app_id;

        let public_key = {
            let keys = self.app_public_keys.read().await;
            keys.get(app_id)
                .ok_or_else(|| anyhow!("App {} not found", app_id))?
                .clone()
        };

        let verified =
            decode::<MCPInstallationClaims>(token, &public_key, &Validation::new(self.algorithm))
                .map_err(|e| anyhow!("Token verification failed: {}", e))?;

        Ok(verified.claims)
    }

    /// Try to decode token as app token
    async fn try_decode_as_app_token(&self, token: &str) -> Result<MCPAppClaims> {
        let unverified: jsonwebtoken::TokenData<MCPAppClaims> =
            decode(token, &DecodingKey::from_secret(&[]), &{
                let mut validation = Validation::new(self.algorithm);
                validation.insecure_disable_signature_validation();
                validation
            })?;

        let app_id = &unverified.claims.iss;

        let public_key = {
            let keys = self.app_public_keys.read().await;
            keys.get(app_id)
                .ok_or_else(|| anyhow!("App {} not found", app_id))?
                .clone()
        };

        let verified = decode::<MCPAppClaims>(token, &public_key, &Validation::new(self.algorithm))
            .map_err(|e| anyhow!("Token verification failed: {}", e))?;

        Ok(verified.claims)
    }

    /// Validate session token claims and return MCP token claims
    async fn validate_session_token_claims(
        &self,
        claims: MCPSessionClaims,
    ) -> Result<MCPTokenClaims> {
        // Check if token is revoked
        let revoked = self.revoked_tokens.read().await;
        if revoked.contains_key(&claims.jti) {
            return Err(anyhow!("Token has been revoked"));
        }

        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp <= now {
            return Err(anyhow!("Token has expired"));
        }

        // Update last used time
        if let Some(session_data) = self.session_store.write().await.get_mut(&claims.jti) {
            session_data.last_used = Utc::now();
        }

        Ok(MCPTokenClaims {
            installation_id: claims.installation_id,
            app_id: claims.app_id,
            permissions: self.session_permissions_to_mcp_permissions(&claims.permissions),
            token_type: TokenType::Session,
            session_id: Some(claims.session_id),
            user_id: claims.user_id,
            project_contexts: claims.project_contexts,
        })
    }

    /// Validate installation token claims
    async fn validate_installation_token_claims(
        &self,
        claims: MCPInstallationClaims,
    ) -> Result<MCPTokenClaims> {
        // Check if token is revoked
        let revoked = self.revoked_tokens.read().await;
        if revoked.contains_key(&claims.jti) {
            return Err(anyhow!("Token has been revoked"));
        }

        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp <= now {
            return Err(anyhow!("Token has expired"));
        }

        // Verify installation still exists and is not suspended
        let installation = {
            let store = self.installation_store.read().await;
            store
                .get(&claims.installation_id)
                .ok_or_else(|| anyhow!("Installation {} not found", claims.installation_id))?
                .clone()
        };

        if installation.suspended_at.is_some() {
            return Err(anyhow!(
                "Installation {} is suspended",
                claims.installation_id
            ));
        }

        // Get project contexts from installation
        let project_contexts = installation
            .project_contexts
            .iter()
            .map(|ctx| ctx.context_id.clone())
            .collect();

        Ok(MCPTokenClaims {
            installation_id: claims.installation_id,
            app_id: claims.app_id,
            permissions: claims.permissions,
            token_type: TokenType::Installation,
            session_id: None,
            user_id: None,
            project_contexts,
        })
    }

    /// Validate app token claims
    async fn validate_app_token_claims(&self, claims: MCPAppClaims) -> Result<MCPTokenClaims> {
        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp <= now {
            return Err(anyhow!("Token has expired"));
        }

        // App tokens have minimal permissions
        Ok(MCPTokenClaims {
            installation_id: String::new(), // App tokens don't have installation context
            app_id: claims.iss,
            permissions: MCPPermissions::default(), // Minimal permissions
            token_type: TokenType::App,
            session_id: None,
            user_id: None,
            project_contexts: Vec::new(),
        })
    }

    /// Revoke a token by its JWT ID
    pub async fn revoke_token(&self, token_id: &str) -> Result<()> {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(token_id.to_string(), Utc::now());

        // Also remove from session store if it's a session token
        let mut sessions = self.session_store.write().await;
        sessions.remove(token_id);

        info!("Revoked token: {}", token_id);
        Ok(())
    }

    /// Clean up expired tokens and sessions
    pub async fn cleanup_expired_tokens(&self) -> Result<()> {
        let now = Utc::now();

        // Clean up expired sessions
        {
            let mut sessions = self.session_store.write().await;
            sessions.retain(|_, session| session.expires_at > now);
        }

        // Clean up old revoked tokens (older than 7 days)
        {
            let mut revoked = self.revoked_tokens.write().await;
            let cutoff = now - Duration::days(7);
            revoked.retain(|_, revoked_at| *revoked_at > cutoff);
        }

        debug!("Cleaned up expired tokens and sessions");
        Ok(())
    }

    /// Generate RSA key pair instructions
    /// Returns instructions for external key generation
    pub fn get_key_generation_instructions() -> String {
        r#"
To generate RSA key pairs for MCP apps, use OpenSSL:

1. Generate private key:
   openssl genpkey -algorithm RSA -out private_key.pem -pkcs8 -f4 -pkeyopt rsa_keygen_bits:2048

2. Extract public key:
   openssl rsa -pubout -in private_key.pem -out public_key.pem

3. View the keys:
   cat private_key.pem
   cat public_key.pem

Store these keys securely and provide them when creating MCP apps.
"#
        .to_string()
    }

    /// Create an app with externally generated keys
    pub fn create_app_with_keys(
        app_id: String,
        name: String,
        description: String,
        owner: String,
        private_key_pem: String,
        public_key_pem: String,
    ) -> Result<MCPApp> {
        // Validate the keys by trying to parse them
        EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;

        DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| anyhow!("Invalid public key: {}", e))?;

        Ok(MCPApp {
            app_id: app_id.clone(),
            name,
            description,
            owner,
            homepage_url: None,
            webhook_url: None,
            permissions: MCPPermissions::default(),
            events: vec![],
            private_key: private_key_pem,
            public_key: public_key_pem,
            client_id: format!("client_{}", app_id),
            client_secret: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Load test keys from filesystem for testing purposes
    pub fn load_test_keys() -> Result<(String, String)> {
        let private_key = std::fs::read_to_string("test_keys/test.pem")
            .map_err(|e| anyhow!("Failed to read test private key: {}", e))?;
        let public_key = std::fs::read_to_string("test_keys/test.pub")
            .map_err(|e| anyhow!("Failed to read test public key: {}", e))?;

        Ok((private_key, public_key))
    }

    /// Helper to intersect two permission sets
    fn intersect_permissions(
        &self,
        granted: &MCPPermissions,
        requested: &MCPPermissions,
    ) -> MCPPermissions {
        MCPPermissions {
            workflows: self.intersect_permission_level(&granted.workflows, &requested.workflows),
            agents: self.intersect_permission_level(&granted.agents, &requested.agents),
            functions: self.intersect_permission_level(&granted.functions, &requested.functions),
            external_apis: self
                .intersect_permission_level(&granted.external_apis, &requested.external_apis),
            webhooks: self.intersect_permission_level(&granted.webhooks, &requested.webhooks),
            audit_logs: self.intersect_permission_level(&granted.audit_logs, &requested.audit_logs),
            project_contexts: granted.project_contexts.clone(), // TODO: Implement proper intersection
        }
    }

    /// Helper to intersect individual permission levels
    fn intersect_permission_level(
        &self,
        granted: &PermissionLevel,
        requested: &PermissionLevel,
    ) -> PermissionLevel {
        use PermissionLevel::*;
        match (granted, requested) {
            (None, _) | (_, None) => None,
            (Read, Read) => Read,
            (Read, Write) | (Read, Admin) => Read,
            (Write, Read) => Read,
            (Write, Write) => Write,
            (Write, Admin) => Write,
            (Admin, level) => level.clone(),
        }
    }

    /// Convert session permissions to MCP permissions
    pub fn session_permissions_to_mcp_permissions(
        &self,
        session_perms: &MCPSessionPermissions,
    ) -> MCPPermissions {
        // This is a simplified conversion - in practice you'd have more sophisticated logic
        MCPPermissions {
            workflows: if !session_perms.tools.is_empty() {
                PermissionLevel::Read
            } else {
                PermissionLevel::None
            },
            agents: if !session_perms.tools.is_empty() {
                PermissionLevel::Read
            } else {
                PermissionLevel::None
            },
            functions: PermissionLevel::None,
            external_apis: PermissionLevel::None,
            webhooks: PermissionLevel::None,
            audit_logs: PermissionLevel::None,
            project_contexts: session_perms.project_contexts.values().cloned().collect(),
        }
    }
}

/// Client information for token creation
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for MCPJWTService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_service_creation() {
        let service = MCPJWTService::new();
        assert_eq!(service.algorithm, Algorithm::RS256);
    }

    #[tokio::test]
    async fn test_key_pair_loading() {
        let result = MCPJWTService::load_test_keys();
        assert!(result.is_ok());

        let (private_key, public_key) = result.unwrap();
        assert!(private_key.contains("BEGIN PRIVATE KEY"));
        assert!(public_key.contains("BEGIN PUBLIC KEY"));
    }

    #[tokio::test]
    async fn test_app_creation_with_keys() {
        // Read test keys from filesystem
        let (private_key, public_key) = MCPJWTService::load_test_keys().unwrap();

        let result = MCPJWTService::create_app_with_keys(
            "test-app".to_string(),
            "Test App".to_string(),
            "Test application".to_string(),
            "test-owner".to_string(),
            private_key,
            public_key,
        );

        assert!(result.is_ok());
        let app = result.unwrap();
        assert_eq!(app.app_id, "test-app");
        assert_eq!(app.name, "Test App");
    }

    #[tokio::test]
    async fn test_app_registration() {
        let service = MCPJWTService::new();

        // Read test keys from filesystem
        let (private_key, public_key) = MCPJWTService::load_test_keys().unwrap();

        let app = MCPApp {
            app_id: "test-app".to_string(),
            name: "Test App".to_string(),
            description: "Test application".to_string(),
            owner: "test-owner".to_string(),
            homepage_url: None,
            webhook_url: None,
            permissions: MCPPermissions::default(),
            events: vec![],
            private_key,
            public_key,
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = service.register_app(app).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_permission_intersection() {
        let service = MCPJWTService::new();

        let granted = MCPPermissions {
            workflows: PermissionLevel::Admin,
            agents: PermissionLevel::Write,
            functions: PermissionLevel::Read,
            external_apis: PermissionLevel::None,
            webhooks: PermissionLevel::None,
            audit_logs: PermissionLevel::None,
            project_contexts: vec![],
        };

        let requested = MCPPermissions {
            workflows: PermissionLevel::Write,
            agents: PermissionLevel::Admin,
            functions: PermissionLevel::Write,
            external_apis: PermissionLevel::Read,
            webhooks: PermissionLevel::None,
            audit_logs: PermissionLevel::None,
            project_contexts: vec![],
        };

        let result = service.intersect_permissions(&granted, &requested);

        assert_eq!(result.workflows, PermissionLevel::Write);
        assert_eq!(result.agents, PermissionLevel::Write);
        assert_eq!(result.functions, PermissionLevel::Read);
        assert_eq!(result.external_apis, PermissionLevel::None);
    }

    #[tokio::test]
    async fn test_jwt_authentication_flow() {
        let service = MCPJWTService::new();

        // Load test keys
        let (private_key, public_key) = MCPJWTService::load_test_keys().unwrap();

        // Create and register test app
        let app = MCPJWTService::create_app_with_keys(
            "test-auth-app".to_string(),
            "Test Auth App".to_string(),
            "Test application for JWT auth".to_string(),
            "test-owner".to_string(),
            private_key,
            public_key,
        )
        .unwrap();

        service.register_app(app.clone()).await.unwrap();

        // Create and register test installation
        let installation = MCPInstallation {
            installation_id: "test-installation-123".to_string(),
            app_id: app.app_id.clone(),
            account: MCPAccount {
                id: "test-account-456".to_string(),
                login: "test-user".to_string(),
                account_type: MCPAccountType::User,
                avatar_url: None,
                url: None,
            },
            permissions: MCPPermissions {
                workflows: PermissionLevel::Write,
                agents: PermissionLevel::Read,
                functions: PermissionLevel::None,
                external_apis: PermissionLevel::None,
                webhooks: PermissionLevel::None,
                audit_logs: PermissionLevel::None,
                project_contexts: vec![],
            },
            project_contexts: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            suspended_at: None,
            suspended_by: None,
        };

        service
            .register_installation(installation.clone())
            .await
            .unwrap();

        // Test 1: Generate app JWT
        let app_jwt = service
            .generate_app_jwt(&app.app_id, "mcp-server")
            .await
            .unwrap();
        assert!(!app_jwt.is_empty());

        // Test 2: Create installation token
        let installation_token = service
            .create_installation_token(&app.app_id, &installation.installation_id, None)
            .await
            .unwrap();
        assert!(!installation_token.token.is_empty());
        assert_eq!(installation_token.app_id, app.app_id);
        assert_eq!(
            installation_token.installation_id,
            installation.installation_id
        );

        // Test 3: Validate installation token
        let validated_claims = service
            .validate_token(&installation_token.token)
            .await
            .unwrap();
        assert_eq!(validated_claims.app_id, app.app_id);
        assert_eq!(
            validated_claims.installation_id,
            installation.installation_id
        );
        assert_eq!(validated_claims.token_type, TokenType::Installation);

        // Test 4: Create session token
        let session_permissions = MCPSessionPermissions::default();
        let client_info = ClientInfo {
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test-Agent/1.0".to_string()),
        };

        let session_token = service
            .create_session_token(
                &installation.installation_id,
                "test-session-789",
                Some("test-user-123".to_string()),
                session_permissions,
                vec!["test-context".to_string()],
                client_info,
            )
            .await
            .unwrap();
        assert!(!session_token.is_empty());

        // Test 5: Validate session token
        let session_claims = service.validate_token(&session_token).await.unwrap();
        assert_eq!(session_claims.app_id, app.app_id);
        assert_eq!(session_claims.installation_id, installation.installation_id);
        assert_eq!(session_claims.token_type, TokenType::Session);
        assert_eq!(
            session_claims.session_id,
            Some("test-session-789".to_string())
        );
        assert_eq!(session_claims.user_id, Some("test-user-123".to_string()));

        // Test 6: Token revocation (we would need the JWT ID for this)
        // This would be tested in a more complete integration test
    }
}
