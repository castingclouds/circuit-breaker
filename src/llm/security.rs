//! Security and Authentication for LLM Router
//! 
//! This module provides GitHub Apps-style authentication with secure token management,
//! JWT-based sessions, and fine-grained permissions for LLM operations.

use super::*;
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use ring::{digest, hmac, rand};
use base64::{Engine as _, engine::general_purpose};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Timelike};

/// JWT claims for authentication tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,           // Subject (user/app ID)
    pub iss: String,           // Issuer
    pub aud: String,           // Audience  
    pub exp: u64,              // Expiration time
    pub iat: u64,              // Issued at
    pub jti: String,           // JWT ID
    pub permissions: Vec<Permission>,
    pub project_id: Option<String>,
    pub rate_limits: RateLimitClaims,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Permission types for fine-grained access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    // LLM Operations
    LLMChatCompletion,
    LLMStreamingCompletion,
    LLMFunctionCalling,
    
    // Provider Management
    ProviderRead,
    ProviderWrite,
    ProviderHealthCheck,
    
    // Cost and Usage
    CostRead,
    CostWrite,
    UsageRead,
    UsageWrite,
    
    // Configuration
    ConfigRead,
    ConfigWrite,
    
    // Admin Operations
    AdminUserManagement,
    AdminProviderManagement,
    AdminSystemMetrics,
    
    // Project Context
    ProjectRead,
    ProjectWrite,
    ProjectDelete,
    
    // Custom permissions
    Custom(String),
}

/// Rate limit claims embedded in JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitClaims {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub concurrent_requests: u32,
    pub daily_cost_limit: Option<f64>,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub user_id: String,
    pub permissions: HashSet<Permission>,
    pub project_id: Option<String>,
    pub rate_limits: RateLimitClaims,
    pub token_id: String,
    pub expires_at: DateTime<Utc>,
}

/// Security manager for handling authentication and authorization
pub struct SecurityManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    default_expiry_hours: u64,
    token_storage: Arc<dyn TokenStorage>,
    rate_limiter: Arc<RateLimiter>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(
        secret_key: &[u8],
        issuer: String,
        audience: String,
        token_storage: Arc<dyn TokenStorage>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret_key),
            decoding_key: DecodingKey::from_secret(secret_key),
            issuer,
            audience,
            default_expiry_hours: 24,
            token_storage,
            rate_limiter,
        }
    }

    /// Generate a new authentication token
    pub async fn generate_token(
        &self,
        user_id: String,
        permissions: Vec<Permission>,
        project_id: Option<String>,
        rate_limits: Option<RateLimitClaims>,
        expiry_hours: Option<u64>,
    ) -> Result<String, SecurityError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SecurityError::Internal("Failed to get current time".to_string()))?
            .as_secs();

        let expiry = now + (expiry_hours.unwrap_or(self.default_expiry_hours) * 3600);
        let token_id = Uuid::new_v4().to_string();

        let claims = TokenClaims {
            sub: user_id.clone(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: expiry,
            iat: now,
            jti: token_id.clone(),
            permissions: permissions.clone(),
            project_id: project_id.clone(),
            rate_limits: rate_limits.unwrap_or_else(|| RateLimitClaims {
                requests_per_minute: 100,
                tokens_per_minute: 10000,
                concurrent_requests: 5,
                daily_cost_limit: Some(10.0),
            }),
            metadata: HashMap::new(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| SecurityError::TokenGeneration(e.to_string()))?;

        // Store token metadata for tracking and revocation
        let token_metadata = TokenMetadata {
            token_id: token_id.clone(),
            user_id: user_id.clone(),
            project_id: project_id.clone(),
            permissions: permissions.into_iter().collect(),
            created_at: Utc::now(),
            expires_at: DateTime::from_timestamp(expiry as i64, 0)
                .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(24)),
            last_used: None,
            is_revoked: false,
        };

        self.token_storage.store_token_metadata(token_metadata).await
            .map_err(|e| SecurityError::Storage(e))?;

        Ok(token)
    }

    /// Validate and decode an authentication token
    pub async fn validate_token(&self, token: &str) -> Result<AuthResult, SecurityError> {
        // Decode and validate JWT
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| SecurityError::InvalidToken(e.to_string()))?;

        let claims = token_data.claims;

        // Check if token is revoked
        if self.token_storage.is_token_revoked(&claims.jti).await
            .map_err(|e| SecurityError::Storage(e))? {
            return Err(SecurityError::TokenRevoked);
        }

        // Check rate limits
        if !self.rate_limiter.check_limits(&claims.sub, &claims.rate_limits).await {
            return Err(SecurityError::RateLimitExceeded);
        }

        // Update last used timestamp
        self.token_storage.update_last_used(&claims.jti).await
            .map_err(|e| SecurityError::Storage(e))?;

        Ok(AuthResult {
            user_id: claims.sub,
            permissions: claims.permissions.into_iter().collect(),
            project_id: claims.project_id,
            rate_limits: claims.rate_limits,
            token_id: claims.jti,
            expires_at: DateTime::from_timestamp(claims.exp as i64, 0)
                .unwrap_or_else(|| Utc::now()),
        })
    }

    /// Check if user has specific permission
    pub fn has_permission(auth_result: &AuthResult, permission: &Permission) -> bool {
        auth_result.permissions.contains(permission)
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(auth_result: &AuthResult, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| auth_result.permissions.contains(p))
    }

    /// Revoke a token
    pub async fn revoke_token(&self, token_id: &str) -> Result<(), SecurityError> {
        self.token_storage.revoke_token(token_id).await
            .map_err(|e| SecurityError::Storage(e))
    }

    /// Revoke all tokens for a user
    pub async fn revoke_user_tokens(&self, user_id: &str) -> Result<(), SecurityError> {
        self.token_storage.revoke_user_tokens(user_id).await
            .map_err(|e| SecurityError::Storage(e))
    }

    /// Generate API key for external integrations
    pub async fn generate_api_key(
        &self,
        user_id: String,
        name: String,
        permissions: Vec<Permission>,
        project_id: Option<String>,
    ) -> Result<String, SecurityError> {
        let key_id = Uuid::new_v4().to_string();
        let secret = self.generate_secure_secret();
        
        let api_key = ApiKey {
            id: key_id.clone(),
            user_id: user_id.clone(),
            name,
            key_hash: self.hash_secret(&secret),
            permissions: permissions.into_iter().collect(),
            project_id,
            created_at: Utc::now(),
            last_used: None,
            is_active: true,
        };

        self.token_storage.store_api_key(api_key).await
            .map_err(|e| SecurityError::Storage(e))?;

        // Return the full key in format: cb_<key_id>_<secret>
        Ok(format!("cb_{}_{}", key_id, secret))
    }

    /// Validate API key
    pub async fn validate_api_key(&self, api_key: &str) -> Result<AuthResult, SecurityError> {
        if !api_key.starts_with("cb_") {
            return Err(SecurityError::InvalidApiKey);
        }

        let parts: Vec<&str> = api_key.splitn(3, '_').collect();
        if parts.len() != 3 {
            return Err(SecurityError::InvalidApiKey);
        }

        let key_id = parts[1];
        let secret = parts[2];

        let stored_key = self.token_storage.get_api_key(key_id).await
            .map_err(|e| SecurityError::Storage(e))?
            .ok_or(SecurityError::InvalidApiKey)?;

        if !stored_key.is_active {
            return Err(SecurityError::ApiKeyRevoked);
        }

        // Verify secret
        if !self.verify_secret(secret, &stored_key.key_hash) {
            return Err(SecurityError::InvalidApiKey);
        }

        // Update last used
        self.token_storage.update_api_key_last_used(key_id).await
            .map_err(|e| SecurityError::Storage(e))?;

        Ok(AuthResult {
            user_id: stored_key.user_id,
            permissions: stored_key.permissions,
            project_id: stored_key.project_id,
            rate_limits: RateLimitClaims {
                requests_per_minute: 1000,
                tokens_per_minute: 100000,
                concurrent_requests: 10,
                daily_cost_limit: Some(100.0),
            },
            token_id: stored_key.id,
            expires_at: Utc::now() + chrono::Duration::days(365), // API keys are long-lived
        })
    }

    /// Generate secure random secret
    fn generate_secure_secret(&self) -> String {
        let rng = rand::SystemRandom::new();
        let mut secret_bytes = [0u8; 32];
        rand::SecureRandom::fill(&rng, &mut secret_bytes).unwrap();
        general_purpose::URL_SAFE_NO_PAD.encode(secret_bytes)
    }

    /// Hash secret for storage
    fn hash_secret(&self, secret: &str) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.encoding_key.as_ref());
        let signature = hmac::sign(&key, secret.as_bytes());
        general_purpose::STANDARD.encode(signature.as_ref())
    }

    /// Verify secret against hash
    fn verify_secret(&self, secret: &str, hash: &str) -> bool {
        let computed_hash = self.hash_secret(secret);
        computed_hash == *hash
    }
}

/// Token storage trait for persisting token metadata
#[async_trait::async_trait]
pub trait TokenStorage: Send + Sync {
    async fn store_token_metadata(&self, metadata: TokenMetadata) -> Result<(), String>;
    async fn get_token_metadata(&self, token_id: &str) -> Result<Option<TokenMetadata>, String>;
    async fn is_token_revoked(&self, token_id: &str) -> Result<bool, String>;
    async fn revoke_token(&self, token_id: &str) -> Result<(), String>;
    async fn revoke_user_tokens(&self, user_id: &str) -> Result<(), String>;
    async fn update_last_used(&self, token_id: &str) -> Result<(), String>;
    async fn cleanup_expired_tokens(&self) -> Result<u64, String>;
    
    // API Key management
    async fn store_api_key(&self, api_key: ApiKey) -> Result<(), String>;
    async fn get_api_key(&self, key_id: &str) -> Result<Option<ApiKey>, String>;
    async fn update_api_key_last_used(&self, key_id: &str) -> Result<(), String>;
    async fn revoke_api_key(&self, key_id: &str) -> Result<(), String>;
}

/// Token metadata for tracking and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub token_id: String,
    pub user_id: String,
    pub project_id: Option<String>,
    pub permissions: HashSet<Permission>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_revoked: bool,
}

/// API Key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub permissions: HashSet<Permission>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Rate limiter for API requests
pub struct RateLimiter {
    storage: Arc<dyn RateLimitStorage>,
}

impl RateLimiter {
    pub fn new(storage: Arc<dyn RateLimitStorage>) -> Self {
        Self { storage }
    }

    pub async fn check_limits(&self, user_id: &str, limits: &RateLimitClaims) -> bool {
        let now = Utc::now();
        let minute_start = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
        
        // Check requests per minute
        let current_requests = self.storage
            .get_request_count(user_id, minute_start, now)
            .await
            .unwrap_or(0);
            
        if current_requests >= limits.requests_per_minute {
            return false;
        }

        // Check tokens per minute
        let current_tokens = self.storage
            .get_token_count(user_id, minute_start, now)
            .await
            .unwrap_or(0);
            
        if current_tokens >= limits.tokens_per_minute {
            return false;
        }

        // Check concurrent requests
        let concurrent_requests = self.storage
            .get_concurrent_requests(user_id)
            .await
            .unwrap_or(0);
            
        if concurrent_requests >= limits.concurrent_requests {
            return false;
        }

        // Check daily cost limit if specified
        if let Some(daily_limit) = limits.daily_cost_limit {
            let day_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap()
                .and_local_timezone(Utc).unwrap();
            let daily_cost = self.storage
                .get_daily_cost(user_id, day_start, now)
                .await
                .unwrap_or(0.0);
                
            if daily_cost >= daily_limit {
                return false;
            }
        }

        true
    }

    pub async fn record_request(&self, user_id: &str, tokens_used: u32) -> Result<(), String> {
        self.storage.record_request(user_id, tokens_used).await
    }

    pub async fn record_cost(&self, user_id: &str, cost: f64) -> Result<(), String> {
        self.storage.record_cost(user_id, cost).await
    }

    pub async fn start_request(&self, user_id: &str, request_id: &str) -> Result<(), String> {
        self.storage.start_request(user_id, request_id).await
    }

    pub async fn end_request(&self, user_id: &str, request_id: &str) -> Result<(), String> {
        self.storage.end_request(user_id, request_id).await
    }
}

/// Rate limit storage trait
#[async_trait::async_trait]
pub trait RateLimitStorage: Send + Sync {
    async fn get_request_count(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32, String>;
    async fn get_token_count(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32, String>;
    async fn get_concurrent_requests(&self, user_id: &str) -> Result<u32, String>;
    async fn get_daily_cost(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<f64, String>;
    async fn record_request(&self, user_id: &str, tokens_used: u32) -> Result<(), String>;
    async fn record_cost(&self, user_id: &str, cost: f64) -> Result<(), String>;
    async fn start_request(&self, user_id: &str, request_id: &str) -> Result<(), String>;
    async fn end_request(&self, user_id: &str, request_id: &str) -> Result<(), String>;
}

/// Security errors
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    
    #[error("Token has been revoked")]
    TokenRevoked,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    
    #[error("Token generation failed: {0}")]
    TokenGeneration(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("API key has been revoked")]
    ApiKeyRevoked,
    
    #[error("Internal security error: {0}")]
    Internal(String),
}

/// In-memory implementations for development/testing

pub struct InMemoryTokenStorage {
    tokens: Arc<RwLock<HashMap<String, TokenMetadata>>>,
    api_keys: Arc<RwLock<HashMap<String, ApiKey>>>,
}

impl InMemoryTokenStorage {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl TokenStorage for InMemoryTokenStorage {
    async fn store_token_metadata(&self, metadata: TokenMetadata) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(metadata.token_id.clone(), metadata);
        Ok(())
    }

    async fn get_token_metadata(&self, token_id: &str) -> Result<Option<TokenMetadata>, String> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(token_id).cloned())
    }

    async fn is_token_revoked(&self, token_id: &str) -> Result<bool, String> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(token_id).map(|t| t.is_revoked).unwrap_or(true))
    }

    async fn revoke_token(&self, token_id: &str) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        if let Some(token) = tokens.get_mut(token_id) {
            token.is_revoked = true;
        }
        Ok(())
    }

    async fn revoke_user_tokens(&self, user_id: &str) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        for token in tokens.values_mut() {
            if token.user_id == user_id {
                token.is_revoked = true;
            }
        }
        Ok(())
    }

    async fn update_last_used(&self, token_id: &str) -> Result<(), String> {
        let mut tokens = self.tokens.write().await;
        if let Some(token) = tokens.get_mut(token_id) {
            token.last_used = Some(Utc::now());
        }
        Ok(())
    }

    async fn cleanup_expired_tokens(&self) -> Result<u64, String> {
        let mut tokens = self.tokens.write().await;
        let now = Utc::now();
        let initial_count = tokens.len();
        
        tokens.retain(|_, token| token.expires_at > now);
        
        Ok((initial_count - tokens.len()) as u64)
    }

    async fn store_api_key(&self, api_key: ApiKey) -> Result<(), String> {
        let mut api_keys = self.api_keys.write().await;
        api_keys.insert(api_key.id.clone(), api_key);
        Ok(())
    }

    async fn get_api_key(&self, key_id: &str) -> Result<Option<ApiKey>, String> {
        let api_keys = self.api_keys.read().await;
        Ok(api_keys.get(key_id).cloned())
    }

    async fn update_api_key_last_used(&self, key_id: &str) -> Result<(), String> {
        let mut api_keys = self.api_keys.write().await;
        if let Some(key) = api_keys.get_mut(key_id) {
            key.last_used = Some(Utc::now());
        }
        Ok(())
    }

    async fn revoke_api_key(&self, key_id: &str) -> Result<(), String> {
        let mut api_keys = self.api_keys.write().await;
        if let Some(key) = api_keys.get_mut(key_id) {
            key.is_active = false;
        }
        Ok(())
    }
}

pub struct InMemoryRateLimitStorage {
    requests: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, u32)>>>>,
    concurrent: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    costs: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
}

impl InMemoryRateLimitStorage {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            concurrent: Arc::new(RwLock::new(HashMap::new())),
            costs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl RateLimitStorage for InMemoryRateLimitStorage {
    async fn get_request_count(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32, String> {
        let requests = self.requests.read().await;
        if let Some(user_requests) = requests.get(user_id) {
            let count = user_requests.iter()
                .filter(|(timestamp, _)| *timestamp >= start && *timestamp <= end)
                .count() as u32;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    async fn get_token_count(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32, String> {
        let requests = self.requests.read().await;
        if let Some(user_requests) = requests.get(user_id) {
            let count: u32 = user_requests.iter()
                .filter(|(timestamp, _)| *timestamp >= start && *timestamp <= end)
                .map(|(_, tokens)| *tokens)
                .sum();
            Ok(count)
        } else {
            Ok(0)
        }
    }

    async fn get_concurrent_requests(&self, user_id: &str) -> Result<u32, String> {
        let concurrent = self.concurrent.read().await;
        Ok(concurrent.get(user_id).map(|s| s.len()).unwrap_or(0) as u32)
    }

    async fn get_daily_cost(&self, user_id: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<f64, String> {
        let costs = self.costs.read().await;
        if let Some(user_costs) = costs.get(user_id) {
            let total: f64 = user_costs.iter()
                .filter(|(timestamp, _)| *timestamp >= start && *timestamp <= end)
                .map(|(_, cost)| *cost)
                .sum();
            Ok(total)
        } else {
            Ok(0.0)
        }
    }

    async fn record_request(&self, user_id: &str, tokens_used: u32) -> Result<(), String> {
        let mut requests = self.requests.write().await;
        let user_requests = requests.entry(user_id.to_string()).or_insert_with(Vec::new);
        user_requests.push((Utc::now(), tokens_used));
        
        // Keep only last 1000 entries per user
        if user_requests.len() > 1000 {
            user_requests.drain(0..100);
        }
        
        Ok(())
    }

    async fn record_cost(&self, user_id: &str, cost: f64) -> Result<(), String> {
        let mut costs = self.costs.write().await;
        let user_costs = costs.entry(user_id.to_string()).or_insert_with(Vec::new);
        user_costs.push((Utc::now(), cost));
        
        // Keep only last 1000 entries per user
        if user_costs.len() > 1000 {
            user_costs.drain(0..100);
        }
        
        Ok(())
    }

    async fn start_request(&self, user_id: &str, request_id: &str) -> Result<(), String> {
        let mut concurrent = self.concurrent.write().await;
        let user_requests = concurrent.entry(user_id.to_string()).or_insert_with(HashSet::new);
        user_requests.insert(request_id.to_string());
        Ok(())
    }

    async fn end_request(&self, user_id: &str, request_id: &str) -> Result<(), String> {
        let mut concurrent = self.concurrent.write().await;
        if let Some(user_requests) = concurrent.get_mut(user_id) {
            user_requests.remove(request_id);
        }
        Ok(())
    }
}