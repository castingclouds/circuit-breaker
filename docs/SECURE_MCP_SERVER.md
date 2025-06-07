# Secure MCP Server Architecture for Circuit Breaker

## Overview

Circuit Breaker implements a secure Model Context Protocol (MCP) server using a GitHub Apps-inspired authentication architecture. This design provides time-limited, scoped access tokens for AI agents to securely interact with external APIs while maintaining audit trails and fine-grained permission controls.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                                        Circuit Breaker MCP Server                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                                                            │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│  │   MCP Server               │    │   JWT Service               │    │  Permission                │   │
│  │   Endpoints                │    │   (Token Mgmt)              │    │  Manager                   │   │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘   │
│                                                                                                            │
├─────────────────────────────────────────────────────────────────┤
│                                        Authentication Layer                                                │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│  │ Installation               │    │ Session                     │    │ Audit                      │   │
│  │ Management                 │    │ Management                  │    │ Logging                    │   │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                                        Workflow Engine                                                     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│  │ Agent                      │    │ Function                    │    │ External API               │   │
│  │ Coordination               │    │ Runner                      │    │ Integration                │   │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                                        External Services                                                   │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐   │
│  │     GitLab                 │    │     GitHub                  │    │    Stripe                  │   │
│  │     (Your Keys)            │    │     (Your Keys)             │    │    (Your Keys)             │   │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## GitHub Apps-Style Authentication Model

### Core Components

#### 1. MCP App Registration

Similar to GitHub Apps, MCP clients must be registered with the Circuit Breaker server:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPApp {
    pub app_id: AppId,
    pub name: String,
    pub description: String,
    pub homepage_url: Option<String>,
    pub webhook_url: Option<String>,
    pub permissions: MCPPermissions,
    pub events: Vec<WebhookEvent>,
    pub private_key: String,        // PEM-encoded private key
    pub public_key: String,         // PEM-encoded public key
    pub client_id: String,          // Public identifier
    pub client_secret: String,      // Secret for OAuth flow
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPPermissions {
    pub workflows: PermissionLevel,
    pub agents: PermissionLevel,
    pub functions: PermissionLevel,
    pub external_apis: HashMap<String, APIPermissions>,
    pub webhooks: PermissionLevel,
    pub audit_logs: PermissionLevel,
    pub project_contexts: Vec<ProjectContextPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextPermission {
    pub context_id: ProjectContextId,
    pub context_type: ProjectContextType,
    pub permissions: ContextPermissions,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectContextType {
    GitLab { project_id: i64, namespace: String },
    GitHub { owner: String, repo: String },
    Combined { contexts: Vec<ProjectContextId> },
    Custom { provider: String, identifier: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPermissions {
    pub read: bool,
    pub write: bool,
    pub admin: bool,
    pub allowed_operations: Vec<String>,
    pub restricted_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_file_size_mb: Option<u64>,
    pub max_search_results: Option<usize>,
    pub rate_limit_per_hour: Option<u64>,
    pub allowed_file_extensions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionLevel {
    None,
    Read,
    Write,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIPermissions {
    pub scopes: Vec<String>,
    pub rate_limits: RateLimitConfig,
    pub allowed_endpoints: Vec<String>,
    pub forbidden_patterns: Vec<String>,
}
```

#### 2. Installation Management

MCP Apps are "installed" to specific organizations or user accounts:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPInstallation {
    pub installation_id: InstallationId,
    pub app_id: AppId,
    pub account: Account,
    pub permissions: MCPPermissions,
    pub events: Vec<WebhookEvent>,
    pub access_tokens_url: String,
    pub repositories_url: Option<String>,
    pub html_url: String,
    pub app_slug: String,
    pub target_id: i64,
    pub target_type: String,
    pub project_contexts: Vec<ProjectContext>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
    pub suspended_by: Option<UserId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub context_id: ProjectContextId,
    pub name: String,
    pub description: Option<String>,
    pub context_type: ProjectContextType,
    pub configuration: ProjectContextConfig,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextConfig {
    pub base_url: Option<String>,
    pub api_version: Option<String>,
    pub default_branch: Option<String>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_depth: Option<usize>,
    pub cache_duration_hours: Option<u64>,
}

pub type ProjectContextId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: UserId,
    pub login: String,
    pub account_type: AccountType,
    pub avatar_url: Option<String>,
    pub gravatar_id: Option<String>,
    pub url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType {
    User,
    Organization,
}
```

## Token Storage Architecture

### Storage Strategy

Circuit Breaker uses a hybrid storage approach for security tokens:

1. **Persistent Storage**: PostgreSQL for long-lived tokens (app keys, installation credentials)
2. **NATS JetStream**: Session tokens, rotation events, real-time token validation, and high-performance caching

```rust
pub struct TokenStorageManager {
    persistent_store: Arc<dyn PersistentTokenStorage>,
    session_store: Arc<NATSSessionStore>,
    rotation_scheduler: Arc<TokenRotationScheduler>,
}

#[async_trait]
pub trait PersistentTokenStorage {
    async fn store_app_credentials(&self, app_id: &AppId, credentials: &AppCredentials) -> Result<()>;
    async fn store_installation(&self, installation: &MCPInstallation) -> Result<()>;
    async fn get_app_private_key(&self, app_id: &AppId) -> Result<Option<String>>;
    async fn rotate_app_keys(&self, app_id: &AppId, new_keys: &KeyPair) -> Result<()>;
}

#[async_trait]
pub trait SessionTokenStorage {
    async fn store_session_token(&self, token_id: &TokenId, token: &SessionTokenData) -> Result<()>;
    async fn validate_session_token(&self, token: &str) -> Result<Option<SessionTokenData>>;
    async fn revoke_session_token(&self, token_id: &TokenId) -> Result<()>;
    async fn cleanup_expired_tokens(&self) -> Result<u64>;
}

pub struct NATSSessionStore {
    jetstream: Context,
    stream_name: String,
    session_kv_store: Store,
    token_cache_kv_store: Store,
}

impl NATSSessionStore {
    pub async fn new(nats_client: async_nats::Client) -> Result<Self> {
        let jetstream = async_nats::jetstream::new(nats_client);
        
        // Create KV store for session tokens
        let session_kv_store = jetstream
            .create_key_value(kv::Config {
                bucket: "session_tokens".to_string(),
                max_age: Duration::from_hours(1), // Auto-expire session tokens
                storage: StorageType::Memory, // Fast access for session validation
                ..Default::default()
            })
            .await?;
            
        // Create KV store for token validation cache
        let token_cache_kv_store = jetstream
            .create_key_value(kv::Config {
                bucket: "token_cache".to_string(),
                max_age: Duration::from_minutes(15), // Short-lived cache for performance
                storage: StorageType::Memory, // High-performance caching
                ..Default::default()
            })
            .await?;
            
        // Create stream for token rotation events
        let stream_config = stream::Config {
            name: "TOKEN_ROTATION_EVENTS".to_string(),
            subjects: vec![
                "token.rotation.scheduled".to_string(),
                "token.rotation.completed".to_string(),
                "token.revocation.requested".to_string(),
            ],
            retention: stream::RetentionPolicy::Limits,
            max_age: Duration::from_days(30),
            storage: stream::StorageType::File,
            ..Default::default()
        };
        
        jetstream.create_stream(stream_config).await?;
        
        Ok(Self {
            jetstream,
            stream_name: "TOKEN_ROTATION_EVENTS".to_string(),
            session_kv_store,
            token_cache_kv_store,
        })
    }
    
    pub async fn cache_session_token(&self, token_hash: &str, session_data: &SessionTokenData) -> Result<()> {
        let key = format!("token:{}", token_hash);
        let value = serde_json::to_vec(session_data)?;
        
        self.token_cache_kv_store.put(&key, value.into()).await?;
        Ok(())
    }
    
    pub async fn get_cached_session_token(&self, token_hash: &str) -> Result<Option<SessionTokenData>> {
        let key = format!("token:{}", token_hash);
        
        match self.token_cache_kv_store.get(&key).await {
            Ok(Some(entry)) => {
                match serde_json::from_slice::<SessionTokenData>(&entry.value) {
                    Ok(session_data) => {
                        // Check if token is still valid
                        if session_data.expires_at > Utc::now() {
                            Ok(Some(session_data))
                        } else {
                            // Remove expired token from cache
                            let _ = self.token_cache_kv_store.delete(&key).await;
                            Ok(None)
                        }
                    }
                    Err(_) => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
    
    pub async fn invalidate_cached_session_token(&self, token_hash: &str) -> Result<()> {
        let key = format!("token:{}", token_hash);
        self.token_cache_kv_store.delete(&key).await?;
        Ok(())
    }
    
    pub async fn add_to_revocation_list(&self, token_id: &TokenId) -> Result<()> {
        let key = format!("revoked:{}", token_id);
        let value = b"revoked";
        
        // Set with TTL (24 hours) by using a separate KV store with max_age
        self.token_cache_kv_store.put(&key, value.to_vec().into()).await?;
        Ok(())
    }
    
    pub async fn is_token_revoked(&self, token_id: &TokenId) -> Result<bool> {
        let key = format!("revoked:{}", token_id);
        
        match self.token_cache_kv_store.get(&key).await {
            Ok(Some(_)) => Ok(true),
            _ => Ok(false),
        }
    }
}

pub struct TokenRotationScheduler {
    nats_session_store: Arc<NATSSessionStore>,
    persistent_store: Arc<dyn PersistentTokenStorage>,
    rotation_interval: Duration,
}

impl TokenRotationScheduler {
    pub async fn schedule_rotation(&self, app_id: &AppId, rotation_time: DateTime<Utc>) -> Result<()> {
        let rotation_event = TokenRotationEvent {
            app_id: app_id.clone(),
            scheduled_time: rotation_time,
            event_type: RotationEventType::Scheduled,
            created_at: Utc::now(),
        };
        
        // Publish rotation event to NATS stream
        self.nats_session_store.jetstream
            .publish("token.rotation.scheduled", serde_json::to_vec(&rotation_event)?.into())
            .await?;
            
        Ok(())
    }
    
    pub async fn execute_rotation(&self, app_id: &AppId) -> Result<KeyPair> {
        // Generate new key pair
        let new_keys = self.generate_new_key_pair()?;
        
        // Store in persistent storage
        self.persistent_store.rotate_app_keys(app_id, &new_keys).await?;
        
        // Invalidate all session tokens for this app
        self.invalidate_app_session_tokens(app_id).await?;
        
        // Publish completion event
        let completion_event = TokenRotationEvent {
            app_id: app_id.clone(),
            scheduled_time: Utc::now(),
            event_type: RotationEventType::Completed,
            created_at: Utc::now(),
        };
        
        self.nats_session_store.jetstream
            .publish("token.rotation.completed", serde_json::to_vec(&completion_event)?.into())
            .await?;
            
        Ok(new_keys)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokenData {
    pub token_id: TokenId,
    pub installation_id: InstallationId,
    pub app_id: AppId,
    pub permissions: MCPPermissions,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRotationEvent {
    pub app_id: AppId,
    pub scheduled_time: DateTime<Utc>,
    pub event_type: RotationEventType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationEventType {
    Scheduled,
    Completed,
    Failed,
}
```

### Storage Benefits

**PostgreSQL for Persistent Data:**
- App credentials and installation data
- Long-term audit logs
- Key rotation history
- ACID compliance for critical operations

**NATS for Session Management and Caching:**
- Real-time token validation with dual KV stores
- Automatic expiration via TTL on both session and cache stores
- Token rotation event coordination via streams
- High-performance session lookups (memory-based KV stores)
- Recently revoked token blacklist with TTL
- Rate limiting counters and session state management
- Distributed caching across multiple server instances

## Authentication Flow Sequence Diagrams

### 1. App Registration and Installation Flow

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant CB as Circuit Breaker
    participant DB as PostgreSQL
    participant Org as Organization
    
    Note over Dev,Org: App Registration Phase
    Dev->>CB: POST /api/v1/mcp/apps (create app)
    CB->>CB: Generate RSA key pair
    CB->>DB: Store app credentials
    CB-->>Dev: Return app_id, client_id, private_key
    
    Note over Dev,Org: Installation Phase
    Org->>CB: Install app for organization
    CB->>DB: Create installation record
    CB->>CB: Generate installation secrets
    CB-->>Org: Installation completed
    
    Note over Dev,Org: App can now authenticate
```

### 2. Complete Authentication Flow

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant CB as Circuit Breaker
    participant NATS as NATS KV Store
    participant DB as PostgreSQL
    participant Cache as NATS KV Cache
    
    Note over Client,Cache: Authentication Phase
    Client->>Client: Generate App JWT (private key)
    Client->>CB: POST /access_tokens (App JWT + Installation ID)
    
    CB->>DB: Verify app credentials
    CB->>DB: Verify installation exists
    CB->>CB: Generate session token (1-hour TTL)
    CB->>NATS: Store session token (auto-expire)
    CB->>Cache: Cache session for fast validation
    CB-->>Client: Return session access token
    
    Note over Client,Cache: API Request Phase
    Client->>CB: MCP Request (Bearer session token)
    CB->>Cache: Check token cache (fast path)
    alt Token in cache
        Cache-->>CB: Valid session data
    else Token not in cache
        CB->>NATS: Validate session token
        NATS-->>CB: Session data or expired
        CB->>Cache: Cache result for future requests
    end
    
    CB->>CB: Check permissions for requested operation
    CB->>CB: Execute MCP operation
    CB->>CB: Log audit event
    CB-->>Client: MCP Response
    
    Note over Client,Cache: Token Expiration
    NATS->>NATS: Auto-expire session token (TTL)
    Cache->>Cache: Auto-expire cached session (TTL)
```

### 3. Token Rotation Flow

```mermaid
sequenceDiagram
    participant Scheduler as Rotation Scheduler
    participant CB as Circuit Breaker
    participant NATS as NATS Stream
    participant DB as PostgreSQL
    participant Client as MCP Clients
    
    Note over Scheduler,Client: Scheduled Rotation
    Scheduler->>NATS: Publish rotation.scheduled event
    Scheduler->>CB: Trigger key rotation for app
    
    CB->>CB: Generate new RSA key pair
    CB->>DB: Store new keys, mark old as deprecated
    CB->>NATS: Publish rotation.completed event
    CB->>NATS: Invalidate all session tokens for app
    
    Note over Scheduler,Client: Client Re-authentication
    Client->>CB: MCP Request (old session token)
    CB->>NATS: Validate session token
    NATS-->>CB: Token invalidated/expired
    CB-->>Client: 401 Unauthorized
    
    Client->>Client: Generate new App JWT (new private key)
    Client->>CB: POST /access_tokens (new App JWT)
    CB->>CB: Validate with new public key
    CB->>NATS: Create new session token
    CB-->>Client: New session access token
    
    Client->>CB: Retry MCP Request (new session token)
    CB-->>Client: Success
```

### 4. Project Context Access Flow

```mermaid
sequenceDiagram
    participant Client as MCP Client
    participant CB as Circuit Breaker
    participant PCM as Project Context Manager
    participant GitLab as GitLab API
    participant Cache as Context Cache
    
    Note over Client,Cache: Context-Aware Operation
    Client->>CB: search_in_context (with session token)
    CB->>CB: Validate session token
    CB->>PCM: Validate context access permissions
    
    PCM->>Cache: Check cached project structure
    alt Cache hit
        Cache-->>PCM: Return cached structure
    else Cache miss
        PCM->>GitLab: Fetch project structure
        GitLab-->>PCM: Project files and metadata
        PCM->>Cache: Cache structure (TTL)
    end
    
    PCM->>GitLab: Search within project scope
    GitLab-->>PCM: Search results
    PCM->>PCM: Apply resource limits
    PCM-->>CB: Filtered search results
    CB->>CB: Log context access audit event
    CB-->>Client: Search results within context
```

### Authentication Flow

#### 1. JWT Token Generation

```rust
pub struct MCPJWTService {
    private_keys: HashMap<AppId, RsaPrivateKey>,
    jwt_validator: JwtValidator,
    installation_store: Arc<dyn InstallationStorage>,
}

impl MCPJWTService {
    pub fn new(
        persistent_store: Arc<dyn PersistentTokenStorage>,
        session_store: Arc<NATSSessionStore>,
    ) -> Self {
        Self {
            persistent_store,
            session_store,
            jwt_validator: JwtValidator::new(),
        }
    }
    
    /// Generate JWT for MCP app authentication (similar to GitHub Apps JWT)
    pub async fn generate_app_jwt(&self, app_id: &AppId) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::minutes(10); // Short-lived app JWT
        
        let claims = MCPAppClaims {
            iss: app_id.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            aud: "circuit-breaker-mcp".to_string(),
        };
        
        // Get private key from persistent storage
        let private_key_pem = self.persistent_store
            .get_app_private_key(app_id)
            .await?
            .ok_or(MCPError::AppNotFound)?;
            
        let token = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(private_key_pem.as_bytes())?,
        )?;
        
        Ok(token)
    }
    
    /// Exchange app JWT for installation access token
    pub async fn create_installation_token(
        &self,
        app_jwt: &str,
        installation_id: &InstallationId,
        permissions: Option<MCPPermissions>,
        client_info: ClientInfo,
    ) -> Result<InstallationToken> {
        // Verify app JWT
        let app_claims = self.verify_app_jwt(app_jwt).await?;
        
        // Verify installation belongs to this app
        let installation = self.persistent_store
            .get_installation(installation_id)
            .await?
            .ok_or(MCPError::InstallationNotFound)?;
            
        if installation.app_id != app_claims.iss {
            return Err(MCPError::InstallationMismatch);
        }
        
        // Generate installation access token
        let token_id = TokenId::new();
        let now = Utc::now();
        let expires_at = now + Duration::hours(1); // 1-hour session tokens
        
        let token_claims = InstallationTokenClaims {
            iss: "circuit-breaker-mcp".to_string(),
            sub: installation_id.clone(),
            app_id: installation.app_id.clone(),
            installation_id: installation_id.clone(),
            permissions: permissions.unwrap_or(installation.permissions),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: token_id.clone(),
        };
        
        // Use HMAC for session tokens (faster validation)
        let installation_secret = self.get_installation_secret(installation_id).await?;
        let token = encode(
            &Header::new(Algorithm::HS256),
            &token_claims,
            &EncodingKey::from_secret(installation_secret.as_bytes()),
        )?;
        
        // Store session data in NATS KV store with TTL
        let session_data = SessionTokenData {
            token_id: token_id.clone(),
            installation_id: installation_id.clone(),
            app_id: installation.app_id.clone(),
            permissions: token_claims.permissions.clone(),
            created_at: now,
            expires_at,
            ip_address: client_info.ip_address,
            user_agent: client_info.user_agent,
        };
        
        self.session_store.store_session_token(&token_id, &session_data).await?;
        
        // Cache in NATS KV for fast validation
        let token_hash = self.hash_token(&token);
        self.session_store.cache_session_token(&token_hash, &session_data).await?;
        
        Ok(InstallationToken {
            token,
            expires_at,
            permissions: token_claims.permissions,
            installation_id: installation_id.clone(),
            app_id: installation.app_id,
        })
    }
    
    /// Fast token validation using cache-first approach
    pub async fn validate_session_token(&self, token: &str) -> Result<SessionTokenData> {
        let token_hash = self.hash_token(token);
        
        // Try cache first (fast lookup)
        if let Some(session_data) = self.session_store.get_cached_session_token(&token_hash).await? {
            if session_data.expires_at > Utc::now() {
                return Ok(session_data);
            }
        }
        
        // Fallback to main session store
        let claims = self.decode_session_token(token).await?;
        
        if let Some(session_data) = self.session_store.validate_session_token(token).await? {
            if session_data.expires_at > Utc::now() {
                // Update cache for future requests
                self.session_store.cache_session_token(&token_hash, &session_data).await?;
                return Ok(session_data);
            }
        }
        
        Err(MCPError::TokenExpired)
    }
    
    /// Revoke session token immediately
    pub async fn revoke_session_token(&self, token_id: &TokenId, token: &str) -> Result<()> {
        // Remove from NATS KV store
        self.session_store.revoke_session_token(token_id).await?;
        
        // Remove from cache
        let token_hash = self.hash_token(token);
        self.session_store.invalidate_cached_session_token(&token_hash).await?;
        
        // Add to revocation list with TTL
        self.session_store.add_to_revocation_list(token_id).await?;
        
        Ok(())
    }
    
    fn hash_token(&self, token: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPAppClaims {
    pub iss: AppId,           // Issuer (App ID)
    pub iat: i64,             // Issued at
    pub exp: i64,             // Expires at
    pub aud: String,          // Audience
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationTokenClaims {
    pub iss: String,          // Issuer (Circuit Breaker)
    pub sub: InstallationId,  // Subject (Installation ID)
    pub app_id: AppId,        // App ID
    pub installation_id: InstallationId,
    pub permissions: MCPPermissions,
    pub iat: i64,             // Issued at
    pub exp: i64,             // Expires at
    pub jti: TokenId,         // JWT ID (unique token identifier)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub permissions: MCPPermissions,
    pub installation_id: InstallationId,
    pub app_id: AppId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}


```

## MCP Server Implementation

### Server Architecture

```rust
pub struct CircuitBreakerMCPServer {
    jwt_service: Arc<MCPJWTService>,
    workflow_engine: Arc<WorkflowEngine>,
    agent_engine: Arc<AgentEngine>,
    function_runner: Arc<FunctionRunner>,
    external_api_client: Arc<ExternalAPIClient>,
    audit_logger: Arc<AuditLogger>,
    session_manager: Arc<SessionManager>,
    project_context_manager: Arc<ProjectContextManager>,
}

pub struct ProjectContextManager {
    contexts: RwLock<HashMap<ProjectContextId, ProjectContext>>,
    context_cache: Arc<ContextCache>,
    installation_store: Arc<dyn InstallationStorage>,
}

impl ProjectContextManager {
    pub async fn get_context(&self, context_id: &ProjectContextId) -> Result<Option<ProjectContext>> {
        let contexts = self.contexts.read().await;
        Ok(contexts.get(context_id).cloned())
    }
    
    pub async fn create_context(
        &self,
        installation_id: &InstallationId,
        context: ProjectContext,
    ) -> Result<ProjectContext> {
        // Verify installation has permission to create this context type
        let installation = self.installation_store
            .get_installation(installation_id)
            .await?
            .ok_or(MCPError::InstallationNotFound)?;
            
        self.validate_context_permissions(&installation, &context)?;
        
        let mut contexts = self.contexts.write().await;
        contexts.insert(context.context_id.clone(), context.clone());
        
        // Initialize context cache
        self.context_cache.initialize_context(&context).await?;
        
        Ok(context)
    }
    
    pub async fn list_contexts_for_installation(
        &self,
        installation_id: &InstallationId,
    ) -> Result<Vec<ProjectContext>> {
        let installation = self.installation_store
            .get_installation(installation_id)
            .await?
            .ok_or(MCPError::InstallationNotFound)?;
            
        Ok(installation.project_contexts)
    }
    
    pub async fn validate_context_access(
        &self,
        session: &MCPSession,
        context_id: &ProjectContextId,
        operation: &str,
    ) -> Result<()> {
        let context = self.get_context(context_id).await?
            .ok_or(MCPError::ContextNotFound)?;
            
        // Check if session has permission for this context
        let has_permission = session.permissions.project_contexts.iter()
            .any(|pcp| {
                pcp.context_id == *context_id && 
                pcp.permissions.allowed_operations.contains(&operation.to_string())
            });
            
        if !has_permission {
            return Err(MCPError::InsufficientContextPermissions);
        }
        
        Ok(())
    }
}

pub struct ContextCache {
    redis_client: Arc<redis::Client>,
    file_cache: Arc<FileCache>,
}

impl ContextCache {
    pub async fn initialize_context(&self, context: &ProjectContext) -> Result<()> {
        match &context.context_type {
            ProjectContextType::GitLab { project_id, .. } => {
                self.cache_gitlab_project_structure(*project_id, context).await
            }
            ProjectContextType::GitHub { owner, repo } => {
                self.cache_github_repo_structure(owner, repo, context).await
            }
            ProjectContextType::Combined { contexts } => {
                for ctx_id in contexts {
                    if let Some(sub_context) = self.get_context_by_id(ctx_id).await? {
                        self.initialize_context(&sub_context).await?;
                    }
                }
                Ok(())
            }
            ProjectContextType::Custom { .. } => {
                // Custom context initialization
                Ok(())
            }
        }
    }
    
    async fn cache_gitlab_project_structure(
        &self,
        project_id: i64,
        context: &ProjectContext,
    ) -> Result<()> {
        // Cache project file tree, recent commits, issues, MRs, etc.
        let cache_key = format!("context:gitlab:{}:structure", project_id);
        let structure = self.fetch_gitlab_project_structure(project_id, context).await?;
        
        let mut conn = self.redis_client.get_async_connection().await?;
        let _: () = conn.set_ex(
            &cache_key,
            serde_json::to_string(&structure)?,
            context.configuration.cache_duration_hours.unwrap_or(24) * 3600,
        ).await?;
        
        Ok(())
    }
}

#[async_trait]
impl MCPServer for CircuitBreakerMCPServer {
    async fn handle_request(&self, request: MCPRequest) -> Result<MCPResponse> {
        // Authenticate and authorize request
        let session = self.authenticate_request(&request).await?;
        
        // Log request for audit
        self.audit_logger.log_request(&session, &request).await;
        
        // Route to appropriate handler
        match &request.method {
            MCPMethod::Ping => self.handle_ping(&session, request).await,
            MCPMethod::Initialize => self.handle_initialize(&session, request).await,
            MCPMethod::ListTools => self.handle_list_tools(&session, request).await,
            MCPMethod::CallTool => self.handle_call_tool(&session, request).await,
            MCPMethod::ListResources => self.handle_list_resources(&session, request).await,
            MCPMethod::ReadResource => self.handle_read_resource(&session, request).await,
            MCPMethod::ListPrompts => self.handle_list_prompts(&session, request).await,
            MCPMethod::GetPrompt => self.handle_get_prompt(&session, request).await,
            MCPMethod::SetLoggingLevel => self.handle_set_logging_level(&session, request).await,
            // Custom Circuit Breaker methods
            MCPMethod::CreateWorkflow => self.handle_create_workflow(&session, request).await,
            MCPMethod::ExecuteAgent => self.handle_execute_agent(&session, request).await,
            MCPMethod::CallExternalAPI => self.handle_call_external_api(&session, request).await,
            // Project context methods
            MCPMethod::CreateProjectContext => self.handle_create_project_context(&session, request).await,
            MCPMethod::ListProjectContexts => self.handle_list_project_contexts(&session, request).await,
            MCPMethod::SearchInContext => self.handle_search_in_context(&session, request).await,
            MCPMethod::GetContextFile => self.handle_get_context_file(&session, request).await,
            MCPMethod::ListContextFiles => self.handle_list_context_files(&session, request).await,
        }
    }
    
    async fn authenticate_request(&self, request: &MCPRequest) -> Result<MCPSession> {
        let auth_header = request.headers.get("Authorization")
            .ok_or(MCPError::MissingAuthorization)?;
            
        let token = auth_header.strip_prefix("Bearer ")
            .ok_or(MCPError::InvalidAuthorizationFormat)?;
            
        // Fast token validation using cache-first approach
        let session_data = self.jwt_service.validate_session_token(token).await?;
        
        // Create session from validated data
        let session = MCPSession {
            installation_id: session_data.installation_id,
            app_id: session_data.app_id,
            permissions: session_data.permissions,
            token_id: session_data.token_id,
            expires_at: session_data.expires_at,
            user_agent: request.headers.get("User-Agent").cloned(),
            ip_address: request.headers.get("X-Forwarded-For").cloned(),
        };
        
        // Check if token is revoked
        if self.session_store.is_token_revoked(&session_data.token_id).await? {
            return Err(MCPError::TokenRevoked);
        }
        
        // Update last access time in session manager
        self.session_manager.update_last_access(&session.token_id).await?;
        
        Ok(session)
    }
}

#[derive(Debug, Clone)]
pub struct MCPSession {
    pub installation_id: InstallationId,
    pub app_id: AppId,
    pub permissions: MCPPermissions,
    pub token_id: TokenId,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}
```

### Tool Implementations

#### 1. Workflow Management Tools

```rust
impl CircuitBreakerMCPServer {
    async fn handle_create_workflow(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        // Check permissions
        if !session.permissions.workflows.can_write() {
            return Err(MCPError::InsufficientPermissions);
        }
        
        let params: CreateWorkflowParams = serde_json::from_value(request.params)?;
        
        // Create workflow with installation context
        let workflow = WorkflowDefinition {
            id: WorkflowId::new(),
            name: params.name,
            description: params.description,
            places: params.places,
            transitions: params.transitions,
            initial_place: params.initial_place,
            created_by_installation: Some(session.installation_id.clone()),
            created_by_app: Some(session.app_id.clone()),
            permissions: Some(params.permissions.unwrap_or_default()),
            ..Default::default()
        };
        
        let created_workflow = self.workflow_engine
            .create_workflow(workflow)
            .await?;
            
        // Log creation
        self.audit_logger.log_workflow_created(session, &created_workflow).await;
        
        Ok(MCPResponse::success(json!({
            "workflow_id": created_workflow.id,
            "name": created_workflow.name,
            "status": "created"
        })))
    }
    
    async fn handle_execute_agent(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        // Check permissions
        if !session.permissions.agents.can_write() {
            return Err(MCPError::InsufficientPermissions);
        }
        
        let params: ExecuteAgentParams = serde_json::from_value(request.params)?;
        
        // Verify agent exists and accessible
        let agent = self.agent_engine
            .get_agent(&params.agent_id)
            .await?
            .ok_or(MCPError::AgentNotFound)?;
            
        // Check if installation has permission to use this agent
        if !self.can_access_agent(session, &agent).await? {
            return Err(MCPError::InsufficientPermissions);
        }
        
        // Execute agent with context
        let execution_context = AgentExecutionContext {
            installation_id: session.installation_id.clone(),
            app_id: session.app_id.clone(),
            token_id: session.token_id.clone(),
            requested_by: params.requested_by,
            metadata: params.metadata,
        };
        
        let execution = self.agent_engine
            .execute_agent_with_context(&params.agent_id, &params.input, execution_context)
            .await?;
            
        // Return execution details
        Ok(MCPResponse::success(json!({
            "execution_id": execution.id,
            "status": execution.status,
            "output": execution.output,
            "created_at": execution.created_at
        })))
    }
}
```

#### 2. External API Integration Tools

```rust
impl CircuitBreakerMCPServer {
    async fn handle_call_external_api(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        let params: ExternalAPICallParams = serde_json::from_value(request.params)?;
        
        // Check API permissions
        let api_permissions = session.permissions.external_apis
            .get(&params.service)
            .ok_or(MCPError::ServiceNotAuthorized)?;
            
        // Verify endpoint is allowed
        if !api_permissions.allowed_endpoints.is_empty() {
            let allowed = api_permissions.allowed_endpoints.iter()
                .any(|pattern| self.matches_endpoint_pattern(&params.endpoint, pattern));
            if !allowed {
                return Err(MCPError::EndpointNotAllowed);
            }
        }
        
        // Check forbidden patterns
        for forbidden_pattern in &api_permissions.forbidden_patterns {
            if self.matches_endpoint_pattern(&params.endpoint, forbidden_pattern) {
                return Err(MCPError::EndpointForbidden);
            }
        }
        
        // Apply rate limiting
        self.check_rate_limit(session, &params.service, &api_permissions.rate_limits).await?;
        
        // Make the API call with service credentials
        let api_call = ExternalAPICall {
            service: params.service.clone(),
            endpoint: params.endpoint,
            method: params.method,
            headers: params.headers,
            body: params.body,
            installation_context: Some(InstallationContext {
                installation_id: session.installation_id.clone(),
                app_id: session.app_id.clone(),
                scopes: api_permissions.scopes.clone(),
            }),
        };
        
        let response = self.external_api_client
            .make_call(api_call)
            .await?;
            
        // Log API call for audit
        self.audit_logger.log_external_api_call(session, &params.service, &response).await;
        
        Ok(MCPResponse::success(json!({
            "status": response.status,
            "headers": response.headers,
            "body": response.body,
            "duration_ms": response.duration_ms
        })))
    }
    
    async fn handle_create_project_context(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        let params: CreateProjectContextParams = serde_json::from_value(request.params)?;
        
        // Verify permissions
        if !session.permissions.project_contexts.iter().any(|p| 
            matches!(p.permissions, ContextPermissions { admin: true, .. })
        ) {
            return Err(MCPError::InsufficientPermissions);
        }
        
        let context = ProjectContext {
            context_id: params.context_id.unwrap_or_else(|| format!("ctx_{}", Uuid::new_v4())),
            name: params.name,
            description: params.description,
            context_type: params.context_type,
            configuration: params.configuration.unwrap_or_default(),
            metadata: params.metadata.unwrap_or_default(),
            created_at: Utc::now(),
            last_accessed: None,
        };
        
        let created_context = self.project_context_manager
            .create_context(&session.installation_id, context)
            .await?;
            
        self.audit_logger.log_context_created(session, &created_context).await;
        
        Ok(MCPResponse::success(json!({
            "context_id": created_context.context_id,
            "name": created_context.name,
            "context_type": created_context.context_type,
            "status": "created"
        })))
    }
    
    async fn handle_search_in_context(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        let params: SearchInContextParams = serde_json::from_value(request.params)?;
        
        // Validate context access
        self.project_context_manager
            .validate_context_access(session, &params.context_id, "search")
            .await?;
            
        let context = self.project_context_manager
            .get_context(&params.context_id)
            .await?
            .ok_or(MCPError::ContextNotFound)?;
            
        let search_results = match &context.context_type {
            ProjectContextType::GitLab { project_id, .. } => {
                self.search_gitlab_project(*project_id, &params, &context).await?
            }
            ProjectContextType::GitHub { owner, repo } => {
                self.search_github_repo(owner, repo, &params, &context).await?
            }
            ProjectContextType::Combined { contexts } => {
                let mut all_results = Vec::new();
                for ctx_id in contexts {
                    if let Ok(results) = self.search_in_context_recursive(session, ctx_id, &params).await {
                        all_results.extend(results);
                    }
                }
                all_results
            }
            ProjectContextType::Custom { .. } => {
                return Err(MCPError::UnsupportedContextType);
            }
        };
        
        // Apply resource limits
        let limit = session.permissions.project_contexts
            .iter()
            .find(|p| p.context_id == params.context_id)
            .and_then(|p| p.resource_limits.max_search_results)
            .unwrap_or(100);
            
        let limited_results = search_results.into_iter().take(limit).collect::<Vec<_>>();
        
        Ok(MCPResponse::success(json!({
            "results": limited_results,
            "total_found": limited_results.len(),
            "limited": limited_results.len() == limit,
            "context_id": params.context_id
        })))
    }
    
    async fn handle_get_context_file(
        &self,
        session: &MCPSession,
        request: MCPRequest,
    ) -> Result<MCPResponse> {
        let params: GetContextFileParams = serde_json::from_value(request.params)?;
        
        // Validate context access
        self.project_context_manager
            .validate_context_access(session, &params.context_id, "read")
            .await?;
            
        let context = self.project_context_manager
            .get_context(&params.context_id)
            .await?
            .ok_or(MCPError::ContextNotFound)?;
            
        // Check file size limits
        let size_limit = session.permissions.project_contexts
            .iter()
            .find(|p| p.context_id == params.context_id)
            .and_then(|p| p.resource_limits.max_file_size_mb)
            .unwrap_or(10); // 10MB default
            
        let file_content = match &context.context_type {
            ProjectContextType::GitLab { project_id, .. } => {
                self.get_gitlab_file(*project_id, &params.file_path, &params.ref_name, size_limit).await?
            }
            ProjectContextType::GitHub { owner, repo } => {
                self.get_github_file(owner, repo, &params.file_path, &params.ref_name, size_limit).await?
            }
            _ => return Err(MCPError::UnsupportedOperation),
        };
        
        Ok(MCPResponse::success(json!({
            "file_path": params.file_path,
            "content": file_content.content,
            "encoding": file_content.encoding,
            "size": file_content.size,
            "sha": file_content.sha,
            "context_id": params.context_id
        })))
    }
    
    async fn search_gitlab_project(
        &self,
        project_id: i64,
        params: &SearchInContextParams,
        context: &ProjectContext,
    ) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        match params.search_type.as_str() {
            "code" => {
                // Search in repository files
                let code_results = self.gitlab_search_code(project_id, &params.query, context).await?;
                results.extend(code_results);
            }
            "issues" => {
                // Search in issues
                let issue_results = self.gitlab_search_issues(project_id, &params.query, context).await?;
                results.extend(issue_results);
            }
            "merge_requests" => {
                // Search in merge requests
                let mr_results = self.gitlab_search_merge_requests(project_id, &params.query, context).await?;
                results.extend(mr_results);
            }
            "all" => {
                // Search in all types
                let code_results = self.gitlab_search_code(project_id, &params.query, context).await?;
                let issue_results = self.gitlab_search_issues(project_id, &params.query, context).await?;
                let mr_results = self.gitlab_search_merge_requests(project_id, &params.query, context).await?;
                results.extend(code_results);
                results.extend(issue_results);
                results.extend(mr_results);
            }
            _ => return Err(MCPError::InvalidSearchType),
        }
        
        Ok(results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectContextParams {
    pub context_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub context_type: ProjectContextType,
    pub configuration: Option<ProjectContextConfig>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInContextParams {
    pub context_id: ProjectContextId,
    pub query: String,
    pub search_type: String, // "code", "issues", "merge_requests", "all"
    pub filters: Option<HashMap<String, String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContextFileParams {
    pub context_id: ProjectContextId,
    pub file_path: String,
    pub ref_name: Option<String>, // branch, tag, or commit
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub result_type: String,
    pub title: String,
    pub content: String,
    pub url: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub content: String,
    pub encoding: String,
    pub size: usize,
    pub sha: String,
}
    
    /// Example: GitLab API integration
    async fn handle_gitlab_api_call(
        &self,
        session: &MCPSession,
        params: GitLabAPIParams,
    ) -> Result<MCPResponse> {
        // Get GitLab credentials for this installation
        let gitlab_config = self.get_gitlab_config(&session.installation_id).await?;
        
        let client = reqwest::Client::new();
        let response = client
            .request(params.method.into(), &format!("{}/{}", gitlab_config.base_url, params.endpoint))
            .header("Authorization", format!("Bearer {}", gitlab_config.access_token))
            .header("User-Agent", format!("CircuitBreaker-MCP/{}", session.app_id))
            .json(&params.body)
            .send()
            .await?;
            
        let status = response.status();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body: serde_json::Value = response.json().await?;
        
        Ok(MCPResponse::success(json!({
            "gitlab_response": {
                "status": status.as_u16(),
                "headers": headers,
                "body": body
            }
        })))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAPICallParams {
    pub service: String,         // "gitlab", "github", "stripe", etc.
    pub endpoint: String,        // API endpoint path
    pub method: HttpMethod,      // GET, POST, PUT, DELETE
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationContext {
    pub installation_id: InstallationId,
    pub app_id: AppId,
    pub scopes: Vec<String>,
}
```

## API Endpoints

### Authentication Endpoints

```rust
// POST /api/v1/mcp/apps
pub async fn create_app(
    Json(payload): Json<CreateAppRequest>,
    Extension(auth): Extension<AdminAuth>,
) -> Result<Json<MCPApp>> {
    let (private_key, public_key) = generate_rsa_key_pair()?;
    
    let app = MCPApp {
        app_id: AppId::new(),
        name: payload.name,
        description: payload.description,
        homepage_url: payload.homepage_url,
        webhook_url: payload.webhook_url,
        permissions: payload.permissions,
        events: payload.events,
        private_key: private_key.to_pem_pkcs8()?,
        public_key: public_key.to_pem_pkcs1()?,
        client_id: generate_client_id(),
        client_secret: generate_client_secret(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        owner: auth.user_id,
    };
    
    let created_app = app_store.create_app(app).await?;
    Ok(Json(created_app))
}

// POST /api/v1/mcp/apps/{app_id}/installations/{installation_id}/access_tokens
pub async fn create_installation_token(
    Path((app_id, installation_id)): Path<(AppId, InstallationId)>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CreateTokenRequest>,
    Extension(jwt_service): Extension<Arc<MCPJWTService>>,
) -> Result<Json<InstallationToken>> {
    // Extract and verify app JWT from Authorization header
    let auth_header = headers.get("Authorization")
        .ok_or(MCPError::MissingAuthorization)?
        .to_str()?;
        
    let app_jwt = auth_header.strip_prefix("Bearer ")
        .ok_or(MCPError::InvalidAuthorizationFormat)?;
    
    // Extract client information for audit and security
    let client_info = ClientInfo {
        ip_address: Some(addr.ip().to_string()),
        user_agent: headers.get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
    };
    
    // Create installation access token with client context
    let token = jwt_service.create_installation_token(
        app_jwt,
        &installation_id,
        payload.permissions,
        client_info,
    ).await?;
    
    Ok(Json(token))
}

// GET /api/v1/mcp/apps/{app_id}/installations
pub async fn list_installations(
    Path(app_id): Path<AppId>,
    headers: HeaderMap,
) -> Result<Json<Vec<MCPInstallation>>> {
    // Verify app JWT
    let app_jwt = extract_app_jwt(&headers)?;
    jwt_service.verify_app_jwt(&app_jwt)?;
    
    let installations = installation_store.list_by_app(&app_id).await?;
    Ok(Json(installations))
}
```

### MCP Protocol Endpoints

```rust
// POST /mcp/v1/transport/http
pub async fn handle_mcp_request(
    Json(request): Json<MCPRequest>,
    Extension(mcp_server): Extension<Arc<CircuitBreakerMCPServer>>,
) -> Result<Json<MCPResponse>> {
    let response = mcp_server.handle_request(request).await?;
    Ok(Json(response))
}

// WebSocket endpoint for MCP transport
// GET /mcp/v1/transport/ws
pub async fn handle_mcp_websocket(
    ws: WebSocketUpgrade,
    Extension(mcp_server): Extension<Arc<CircuitBreakerMCPServer>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_mcp_ws_connection(socket, mcp_server))
}

async fn handle_mcp_ws_connection(
    socket: WebSocket,
    mcp_server: Arc<CircuitBreakerMCPServer>,
) {
    let (mut sender, mut receiver) = socket.split();
    
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<MCPRequest>(&text) {
                    Ok(request) => {
                        match mcp_server.handle_request(request).await {
                            Ok(response) => {
                                let response_text = serde_json::to_string(&response).unwrap();
                                let _ = sender.send(Message::Text(response_text)).await;
                            }
                            Err(error) => {
                                let error_response = MCPResponse::error(error.to_string());
                                let response_text = serde_json::to_string(&error_response).unwrap();
                                let _ = sender.send(Message::Text(response_text)).await;
                            }
                        }
                    }
                    Err(_) => {
                        let error_response = MCPResponse::error("Invalid JSON".to_string());
                        let response_text = serde_json::to_string(&error_response).unwrap();
                        let _ = sender.send(Message::Text(response_text)).await;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            _ => {}
        }
    }
}
```

## Security Features

### Permission Scoping

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedPermissions {
    pub workflows: WorkflowScopes,
    pub agents: AgentScopes,
    pub external_apis: HashMap<String, APIScopes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowScopes {
    pub create: bool,
    pub read: Vec<WorkflowPattern>,
    pub update: Vec<WorkflowPattern>,
    pub delete: Vec<WorkflowPattern>,
    pub execute: Vec<WorkflowPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    pub name_pattern: Option<String>,
    pub tag_filters: Vec<String>,
    pub owner_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIScopes {
    pub endpoints: Vec<EndpointScope>,
    pub rate_limits: RateLimitConfig,
    pub data_access: DataAccessLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointScope {
    pub pattern: String,        // e.g., "/api/v4/projects/*/issues"
    pub methods: Vec<HttpMethod>,
    pub conditions: Vec<AccessCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataAccessLevel {
    Public,      // Only public data
    Internal,    // Internal project data
    Private,     // Private/sensitive data
    Admin,       // Administrative access
}
```

### Audit Logging

```rust
pub struct AuditLogger {
    storage: Arc<dyn AuditStorage>,
    encryption_key: EncryptionKey,
}

impl AuditLogger {
    pub async fn log_request(&self, session: &MCPSession, request: &MCPRequest) {
        let audit_event = AuditEvent {
            id: AuditEventId::new(),
            timestamp: Utc::now(),
            event_type: AuditEventType::MCPRequest,
            installation_id: session.installation_id.clone(),
            app_id: session.app_id.clone(),
            user_agent: session.user_agent.clone(),
            ip_address: session.ip_address.clone(),
            details: AuditDetails::MCPRequest {
                method: request.method.clone(),
                params: self.sanitize_params(&request.params),
                request_id: request.id.clone(),
            },
            sensitive_data_hash: self.hash_sensitive_data(&request.params),
        };
        
        let _ = self.storage.store_event(audit_event).await;
    }
    
    pub async fn log_external_api_call(
        &self,
        session: &MCPSession,
        service: &str,
        response: &ExternalAPIResponse,
    ) {
        let audit_event = AuditEvent {
            id: AuditEventId::new(),
            timestamp: Utc::now(),
            event_type: AuditEventType::ExternalAPICall,
            installation_id: session.installation_id.clone(),
            app_id: session.app_id.clone(),
            details: AuditDetails::ExternalAPICall {
                service: service.to_string(),
                endpoint: response.endpoint.clone(),
                method: response.method.clone(),
                status_code: response.status,
                duration_ms: response.duration_ms,
                response_size_bytes: response.body.len(),
            },
            ..Default::default()
        };
        
        let _ = self.storage.store_event(audit_event).await;
    }
    
    fn sanitize_params(&self, params: &serde_json::Value) -> serde_json::Value {
        // Remove sensitive information from params for logging
        let mut sanitized = params.clone();
        self.remove_sensitive_fields(&mut sanitized);
        sanitized
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: AuditEventId,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub installation_id: InstallationId,
    pub app_id: AppId,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub details: AuditDetails,
    pub sensitive_data_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    MCPRequest,
    ExternalAPICall,
    TokenCreated,
    TokenRevoked,
    PermissionChanged,
    InstallationCreated,
    InstallationSuspended,
}
```

### Rate Limiting

```rust
pub struct RateLimiter {
    rate_limit_kv_store: Store,
    default_limits: RateLimitConfig,
}

impl RateLimiter {
    pub async fn new(jetstream: &Context) -> Result<Self> {
        let rate_limit_kv_store = jetstream
            .create_key_value(kv::Config {
                bucket: "rate_limits".to_string(),
                max_age: Duration::from_hours(1), // Rate limit windows expire after 1 hour
                storage: StorageType::Memory, // Fast access for rate limiting
                ..Default::default()
            })
            .await?;
            
        Ok(Self {
            rate_limit_kv_store,
            default_limits: RateLimitConfig::default(),
        })
    }
    
    pub async fn check_rate_limit(
        &self,
        session: &MCPSession,
        resource: &str,
        limits: &RateLimitConfig,
    ) -> Result<RateLimitResult> {
        let window_start = Utc::now().timestamp() / limits.window_seconds;
        let window_key = format!("rate_limit:{}:{}:{}", 
            session.installation_id, resource, window_start);
        
        // Get current count from NATS KV store
        let current = match self.rate_limit_kv_store.get(&window_key).await {
            Ok(Some(entry)) => {
                String::from_utf8_lossy(&entry.value).parse::<i64>().unwrap_or(0)
            }
            _ => 0,
        };
        
        if current >= limits.max_requests {
            return Ok(RateLimitResult::Limited {
                limit: limits.max_requests,
                current,
                reset_at: DateTime::from_timestamp((window_start + 1) * limits.window_seconds, 0).unwrap(),
            });
        }
        
        // Increment counter in NATS KV store
        let new_count = current + 1;
        let _= self.rate_limit_kv_store.put(&window_key, new_count.to_string().into()).await?;
        
        Ok(RateLimitResult::Allowed {
            limit: limits.max_requests,
            remaining: limits.max_requests - new_count,
            reset_at: DateTime::from_timestamp((window_start + 1) * limits.window_seconds, 0).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: i64,
    pub window_seconds: i64,
    pub burst_allowance: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed {
        limit: i64,
        remaining: i64,
        reset_at: DateTime<Utc>,
    },
    Limited {
        limit: i64,
        current: i64,
        reset_at: DateTime<Utc>,
    },
}
```

## Integration with Webhook Patterns

### GitLab Integration Example

Building on the webhook integration patterns from `WEBHOOK_INTEGRATION_PATTERNS.md`, the MCP server can provide secure access to GitLab APIs:

```rust
pub struct GitLabMCPTool {
    client: GitLabClient,
    installation_store: Arc<dyn InstallationStorage>,
}

impl GitLabMCPTool {
    pub async fn handle_gitlab_request(
        &self,
        session: &MCPSession,
        params: GitLabToolParams,
    ) -> Result<MCPResponse> {
        // Get GitLab configuration for this installation
        let gitlab_config = self.get_gitlab_config(&session.installation_id).await?;
        
        match params.action {
            GitLabAction::ListProjects => {
                let projects = self.client
                    .projects()
                    .list()
                    .with_token(&gitlab_config.access_token)
                    .send()
                    .await?;
                    
                Ok(MCPResponse::success(json!({
                    "projects": projects,
                    "total": projects.len()
                })))
            }
            
            GitLabAction::CreateIssue { project_id, title, description, labels } => {
                let issue = self.client
                    .projects()
                    .issues(project_id)
                    .create()
                    .with_token(&gitlab_config.access_token)
                    .title(&title)
                    .description(&description)
                    .labels(labels)
                    .send()
                    .await?;
                    
                Ok(MCPResponse::success(json!({
                    "issue": issue,
                    "url": format!("{}/issues/{}", issue.web_url, issue.iid)
                })))
            }
            
            GitLabAction::CreateMergeRequest { 
                project_id, 
                source_branch, 
                target_branch, 
                title, 
                description 
            } => {
                let mr = self.client
                    .projects()
                    .merge_requests(project_id)
                    .create()
                    .with_token(&gitlab_config.access_token)
                    .source_branch(&source_branch)
                    .target_branch(&target_branch)
                    .title(&title)
                    .description(&description)
                    .send()
                    .await?;
                    
                Ok(MCPResponse::success(json!({
                    "merge_request": mr,
                    "url": mr.web_url
                })))
            }
            
            GitLabAction::AddComment { project_id, issue_iid, body } => {
                let comment = self.client
                    .projects()
                    .issues(project_id)
                    .notes(issue_iid)
                    .create()
                    .with_token(&gitlab_config.access_token)
                    .body(&body)
                    .send()
                    .await?;
                    
                Ok(MCPResponse::success(json!({
                    "comment": comment,
                    "created_at": comment.created_at
                })))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitLabAction {
    ListProjects,
    CreateIssue {
        project_id: i64,
        title: String,
        description: String,
        labels: Vec<String>,
    },
    CreateMergeRequest {
        project_id: i64,
        source_branch: String,
        target_branch: String,
        title: String,
        description: String,
    },
    AddComment {
        project_id: i64,
        issue_iid: i64,
        body: String,
    },
}
```

### Webhook Event Processing

```rust
pub struct WebhookProcessor {
    mcp_server: Arc<CircuitBreakerMCPServer>,
    event_bus: Arc<EventBus>,
}

impl WebhookProcessor {
    /// Process incoming webhook and potentially trigger MCP workflows
    pub async fn process_webhook(
        &self,
        webhook: WebhookEvent,
        installation_id: &InstallationId,
    ) -> Result<()> {
        match webhook.event_type {
            WebhookEventType::GitLabIssueCreated => {
                // Find MCP apps that want to be notified of this event
                let interested_apps = self.find_apps_for_event(
                    installation_id,
                    &webhook.event_type,
                ).await?;
                
                for app in interested_apps {
                    // Create a workflow token with webhook data
                    let token_data = json!({
                        "webhook_event": webhook.clone(),
                        "triggered_by": "webhook",
                        "installation_id": installation_id,
                        "app_id": app.app_id
                    });
                    
                    // Trigger workflow if configured
                    if let Some(workflow_id) = app.webhook_workflow_mapping.get(&webhook.event_type) {
                        self.mcp_server
                            .create_workflow_instance(workflow_id, token_data)
                            .await?;
                    }
                    
                    // Send webhook notification to app
                    if let Some(webhook_url) = &app.webhook_url {
                        self.send_webhook_notification(webhook_url, &webhook, installation_id).await?;
                    }
                }
            }
            
            WebhookEventType::GitLabMergeRequestMerged => {
                // Example: Auto-deploy workflow triggered by MR merge
                self.trigger_deployment_workflow(&webhook, installation_id).await?;
            }
            
            _ => {
                // Handle other webhook types
            }
        }
        
        Ok(())
    }
    
    async fn trigger_deployment_workflow(
        &self,
        webhook: &WebhookEvent,
        installation_id: &InstallationId,
    ) -> Result<()> {
        // Extract merge request details
        let mr_data = webhook.data.as_object()
            .and_then(|obj| obj.get("merge_request"))
            .ok_or(MCPError::InvalidWebhookData)?;
            
        let target_branch = mr_data.get("target_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");
            
        // Only trigger deployment for main branch merges
        if target_branch == "main" {
            let workflow_data = json!({
                "merge_request": mr_data,
                "project_id": webhook.data.get("project").and_then(|p| p.get("id")),
                "deployment_target": "production",
                "triggered_by": "merge_request_merged"
            });
            
            // Create deployment workflow instance
            self.mcp_server
                .create_workflow_instance("auto_deployment", workflow_data)
                .await?;
        }
        
        Ok(())
    }
}
```

## Client SDK Examples

### Python MCP Client

```python
import asyncio
import json
import jwt
import time
from datetime import datetime, timedelta
from typing import Dict, Any, Optional
import httpx
import websockets

class CircuitBreakerMCPClient:
    def __init__(self, config: MCPClientConfig):
        self.config = config
        self.session_token: Optional[str] = None
        self.session_expires: Optional[datetime] = None
        self.http_client = httpx.AsyncClient()
        
    async def authenticate(self) -> str:
        """Generate app JWT and exchange for session token"""
        
        # Generate app JWT
        app_jwt = self._generate_app_jwt()
        
        # Exchange for installation access token
        response = await self.http_client.post(
            f"{self.config.base_url}/api/v1/mcp/apps/{self.config.app_id}/installations/{self.config.installation_id}/access_tokens",
            headers={"Authorization": f"Bearer {app_jwt}"},
            json={"permissions": self.config.permissions}
        )
        
        if response.status_code != 200:
            raise MCPAuthenticationError(f"Failed to authenticate: {response.text}")
            
        token_data = response.json()
        self.session_token = token_data["token"]
        self.session_expires = datetime.fromisoformat(token_data["expires_at"])
        
        return self.session_token
    
    def _generate_app_jwt(self) -> str:
        """Generate JWT for app authentication"""
        now = int(time.time())
        payload = {
            "iss": self.config.app_id,
            "iat": now,
            "exp": now + 600,  # 10 minutes
            "aud": "circuit-breaker-mcp"
        }
        
        return jwt.encode(payload, self.config.private_key, algorithm="RS256")
    
    async def ensure_authenticated(self):
        """Ensure we have a valid session token"""
        if not self.session_token or (self.session_expires and datetime.now() > self.session_expires):
            await self.authenticate()
    
    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool"""
        await self.ensure_authenticated()
        
        request = {
            "jsonrpc": "2.0",
            "id": f"req_{int(time.time())}",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }
        
        response = await self.http_client.post(
            f"{self.config.base_url}/mcp/v1/transport/http",
            headers={"Authorization": f"Bearer {self.session_token}"},
            json=request
        )
        
        if response.status_code != 200:
            raise MCPRequestError(f"Tool call failed: {response.text}")
            
        return response.json()
    
    async def create_workflow(self, name: str, definition: Dict[str, Any]) -> Dict[str, Any]:
        """Create a new workflow"""
        return await self.call_tool("create_workflow", {
            "name": name,
            "definition": definition
        })
    
    async def execute_agent(self, agent_id: str, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """Execute an AI agent"""
        return await self.call_tool("execute_agent", {
            "agent_id": agent_id,
            "input": input_data
        })
    
    async def call_gitlab_api(self, endpoint: str, method: str = "GET", data: Optional[Dict] = None) -> Dict[str, Any]:
        """Call GitLab API through MCP"""
        return await self.call_tool("call_external_api", {
            "service": "gitlab",
            "endpoint": endpoint,
            "method": method,
            "body": data
        })

# Usage example
async def main():
    config = MCPClientConfig(
        base_url="https://circuit-breaker.example.com",
        app_id="your_app_id",
        installation_id="your_installation_id",
        private_key=load_private_key("path/to/private_key.pem"),
        permissions={
            "workflows": "write",
            "agents": "write",
            "external_apis": {
                "gitlab": {
                    "scopes": ["api", "read_user"],
                    "allowed_endpoints": ["/api/v4/projects/*"]
                }
            }
        }
    )
    
    client = CircuitBreakerMCPClient(config)
    
    # Create a workflow that processes GitLab issues
    workflow_result = await client.create_workflow(
        name="issue_analyzer",
        definition={
            "places": ["analyze", "categorize", "respond"],
            "initial_place": "analyze",
            "transitions": [
                {
                    "id": "analyze_to_categorize",
                    "from_places": ["analyze"],
                    "to_place": "categorize",
                    "agent_execution": {
                        "agent_id": "issue_analyzer",
                        "input_mapping": {"issue_text": "data.description"}
                    }
                }
            ]
        }
    )
    
    print(f"Created workflow: {workflow_result}")
    
    # Execute an agent to analyze an issue
    agent_result = await client.execute_agent(
        agent_id="issue_classifier",
        input_data={
            "issue_title": "Bug in authentication system",
            "issue_description": "Users are unable to log in using OAuth",
            "priority": "high"
        }
    )
    
    print(f"Agent analysis: {agent_result}")
    
    # Call GitLab API to get project issues
    gitlab_result = await client.call_gitlab_api(
        endpoint="/api/v4/projects/123/issues",
        method="GET"
    )
    
    print(f"GitLab issues: {gitlab_result}")

if __name__ == "__main__":
    asyncio.run(main())
```

### JavaScript MCP Client

```typescript
import jwt from 'jsonwebtoken';
import WebSocket from 'ws';

interface MCPClientConfig {
  baseUrl: string;
  appId: string;
  installationId: string;
  privateKey: string;
  permissions?: Record<string, any>;
}

class CircuitBreakerMCPClient {
  private sessionToken?: string;
  private sessionExpires?: Date;
  private ws?: WebSocket;
  
  constructor(private config: MCPClientConfig) {}
  
  async authenticate(): Promise<string> {
    // Generate app JWT
    const appJwt = this.generateAppJwt();
    
    // Exchange for installation access token
    const response = await fetch(
      `${this.config.baseUrl}/api/v1/mcp/apps/${this.config.appId}/installations/${this.config.installationId}/access_tokens`,
      {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${appJwt}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          permissions: this.config.permissions
        }),
      }
    );
    
    if (!response.ok) {
      throw new Error(`Authentication failed: ${await response.text()}`);
    }
    
    const tokenData = await response.json();
    this.sessionToken = tokenData.token;
    this.sessionExpires = new Date(tokenData.expires_at);
    
    return this.sessionToken;
  }
  
  private generateAppJwt(): string {
    const now = Math.floor(Date.now() / 1000);
    const payload = {
      iss: this.config.appId,
      iat: now,
      exp: now + 600, // 10 minutes
      aud: 'circuit-breaker-mcp',
    };
    
    return jwt.sign(payload, this.config.privateKey, { algorithm: 'RS256' });
  }
  
  async ensureAuthenticated(): Promise<void> {
    if (!this.sessionToken || (this.sessionExpires && new Date() > this.sessionExpires)) {
      await this.authenticate();
    }
  }
  
  async callTool(toolName: string, arguments: Record<string, any>): Promise<any> {
    await this.ensureAuthenticated();
    
    const request = {
      jsonrpc: '2.0',
      id: `req_${Date.now()}`,
      method: 'tools/call',
      params: {
        name: toolName,
        arguments,
      },
    };
    
    const response = await fetch(`${this.config.baseUrl}/mcp/v1/transport/http`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.sessionToken}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    
    if (!response.ok) {
      throw new Error(`Tool call failed: ${await response.text()}`);
    }
    
    return response.json();
  }
  
  async createWorkflow(name: string, definition: any): Promise<any> {
    return this.callTool('create_workflow', { name, definition });
  }
  
  async executeAgent(agentId: string, inputData: any): Promise<any> {
    return this.callTool('execute_agent', {
      agent_id: agentId,
      input: inputData,
    });
  }
  
  async callGitLabAPI(endpoint: string, method = 'GET', data?: any): Promise<any> {
    return this.callTool('call_external_api', {
      service: 'gitlab',
      endpoint,
      method,
      body: data,
    });
  }
  
  async connectWebSocket(): Promise<WebSocket> {
    await this.ensureAuthenticated();
    
    this.ws = new WebSocket(`${this.config.baseUrl.replace('http', 'ws')}/mcp/v1/transport/ws`, {
      headers: {
        'Authorization': `Bearer ${this.sessionToken}`,
      },
    });
    
    return new Promise((resolve, reject) => {
      this.ws!.on('open', () => resolve(this.ws!));
      this.ws!.on('error', reject);
    });
  }
}

// Usage example
async function main() {
  async function main() {
      const client = new CircuitBreakerMCPClient({
          baseUrl: 'https://circuit-breaker.example.com',
          appId: 'your_app_id',
          installationId: 'your_installation_id',
          privateKey: fs.readFileSync('private_key.pem', 'utf8'),
          permissions: {
              workflows: 'write',
              agents: 'write',
              external_apis: {
                  gitlab: {
                      scopes: ['api', 'read_user'],
                      allowed_endpoints: ['/api/v4/projects/*'],
                  },
              },
              project_contexts: [
                  {
                      context_id: 'myorg_myproject',
                      context_type: { GitLab: { project_id: 123, namespace: 'myorg' } },
                      permissions: { read: true, write: true, admin: false },
                  },
              ],
          },
      });
    
      // Create a project context for focused operations
      const context = await client.callTool('create_project_context', {
          name: 'My Project Context',
          context_type: { GitLab: { project_id: 123, namespace: 'myorg' } },
          configuration: {
              default_branch: 'main',
              include_patterns: ['src/**/*.ts', 'docs/**/*.md'],
              exclude_patterns: ['node_modules/**', '*.log'],
              max_depth: 10,
          },
      });
    
      console.log('Created project context:', context);
    
      // Search within the project context
      const searchResults = await client.callTool('search_in_context', {
          context_id: 'myorg_myproject',
          query: 'authentication performance',
          search_type: 'all',
          limit: 20,
      });
    
      console.log('Search results:', searchResults);
    
      // Get specific file from context
      const fileContent = await client.callTool('get_context_file', {
          context_id: 'myorg_myproject',
          file_path: 'src/auth/authentication.ts',
          ref_name: 'main',
      });
    
      // Analyze the file content with AI
      const analysis = await client.executeAgent('code_analyzer', {
          file_content: fileContent.content,
          file_path: fileContent.file_path,
          context: 'performance optimization',
          search_results: searchResults.results,
      });
    
      console.log('Code analysis:', analysis);
    
      // Create a contextualized comment on GitLab issue
      const comment = await client.callGitLabAPI(
          '/api/v4/projects/123/issues/456/notes',
          'POST',
          {
              body: `AI Analysis based on project context:\n\n${analysis.summary}\n\nRelated files analyzed:\n${analysis.related_files.join('\n')}\n\nRecommended actions:\n${analysis.recommendations.join('\n')}`,
          }
      );
    
      console.log('Created contextualized comment:', comment);
  }

main().catch(console.error);
```

## Deployment and Configuration

### Docker Deployment

```dockerfile
# Dockerfile for Circuit Breaker MCP Server
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin mcp-server

FROM alpine:latest

RUN apk add --no-cache \
    ca-certificates \
    libssl3

COPY --from=builder /app/target/release/mcp-server /usr/local/bin/mcp-server

EXPOSE 4000

CMD ["mcp-server"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  circuit-breaker-mcp:
    build: .
    ports:
      - "4000:4000"
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://user:pass@postgres:5432/circuit_breaker
      - JWT_SECRET=your-jwt-secret-here
      - NATS_URL=nats://nats:4222
    depends_on:
      - postgres
      - nats
    volumes:
      - ./config:/app/config

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=circuit_breaker
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
    volumes:
      - postgres_data:/var/lib/postgresql/data

  nats:
    image: nats:alpine
    command: ["--jetstream", "--http_port", "8222", "--store_dir", "/data"]
    ports:
      - "4222:4222"
      - "8222:8222"
    volumes:
      - nats_data:/data

volumes:
  postgres_data:
  nats_data:
```

### Configuration

```yaml
# config/mcp_server.yml
server:
  host: "0.0.0.0"
  port: 4000
  cors_origins: ["*"]

auth:
  jwt_secret: "${JWT_SECRET}"
  token_expiry_hours: 1
  app_jwt_expiry_minutes: 10
  
database:
  url: "${DATABASE_URL}"
  max_connections: 10
  
redis:
  url: "${REDIS_URL}"
  
nats:
  url: "${NATS_URL}"
  jetstream:
    max_memory: "1GB"
    max_file_storage: "10GB"
  
external_apis:
  gitlab:
    base_url: "https://gitlab.com"
    timeout_seconds: 30
    
  github:
    base_url: "https://api.github.com"
    timeout_seconds: 30
    
rate_limiting:
  default_requests_per_minute: 1000
  burst_allowance: 100
  
audit:
  enabled: true
  retention_days: 90
  sensitive_data_hashing: true

security:
  private_key_path: "/app/config/mcp_private_key.pem"
  public_key_path: "/app/config/mcp_public_key.pem"
  
permissions:
  default_workflow_permissions:
    create: true
    read: true
    update: false
    delete: false
    
  default_agent_permissions:
    execute: true
    configure: false
    
  default_api_permissions:
    rate_limit_per_minute: 100
    allowed_methods: ["GET", "POST"]
```

## Security Best Practices

### Key Management

```rust
pub struct KeyManager {
    private_keys: HashMap<AppId, RsaPrivateKey>,
    public_keys: HashMap<AppId, RsaPublicKey>,
    key_rotation_schedule: HashMap<AppId, DateTime<Utc>>,
}

impl KeyManager {
    pub async fn rotate_app_keys(&mut self, app_id: &AppId) -> Result<KeyPair> {
        // Generate new key pair
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        // Store new keys
        self.private_keys.insert(app_id.clone(), private_key.clone());
        self.public_keys.insert(app_id.clone(), public_key.clone());
        
        // Schedule next rotation
        let next_rotation = Utc::now() + Duration::days(90);
        self.key_rotation_schedule.insert(app_id.clone(), next_rotation);
        
        // Invalidate existing tokens for this app
        self.invalidate_app_tokens(app_id).await?;
        
        Ok(KeyPair {
            private_key: private_key.to_pem_pkcs8()?,
            public_key: public_key.to_pem_pkcs1()?,
        })
    }
    
    pub async fn check_rotation_schedule(&self) -> Vec<AppId> {
        let now = Utc::now();
        self.key_rotation_schedule
            .iter()
            .filter(|(_, &rotation_time)| rotation_time <= now)
            .map(|(app_id, _)| app_id.clone())
            .collect()
    }
}
```

### Monitoring and Alerting

```rust
pub struct SecurityMonitor {
    metrics: Arc<MetricsCollector>,
    alert_manager: Arc<AlertManager>,
}

impl SecurityMonitor {
    pub async fn monitor_authentication_events(&self, event: AuthEvent) {
        match event {
            AuthEvent::FailedAuthentication { app_id, reason, ip_address } => {
                self.metrics.increment_counter("auth_failures", &[
                    ("app_id", app_id.as_str()),
                    ("reason", reason.as_str()),
                ]);
                
                // Check for brute force attacks
                if self.is_brute_force_attack(&ip_address).await {
                    self.alert_manager.send_alert(Alert::BruteForceAttack {
                        ip_address,
                        app_id,
                    }).await;
                }
            }
            
            AuthEvent::SuspiciousActivity { app_id, activity_type, details } => {
                self.alert_manager.send_alert(Alert::SuspiciousActivity {
                    app_id,
                    activity_type,
                    details,
                }).await;
            }
            
            AuthEvent::UnauthorizedAPIAccess { app_id, endpoint, ip_address } => {
                self.alert_manager.send_alert(Alert::UnauthorizedAccess {
                    app_id,
                    endpoint,
                    ip_address,
                }).await;
            }
        }
    }
}
```

## Conclusion

The Circuit Breaker Secure MCP Server provides a comprehensive, GitHub Apps-inspired authentication architecture that enables secure AI agent interactions with external APIs. Key features include:

### Security Features
- **JWT-based Authentication**: Time-limited, scoped access tokens with automatic rotation
- **Hybrid Storage Architecture**: PostgreSQL for persistence, NATS JetStream for sessions and caching
- **Fine-grained Permissions**: Granular control over API access and capabilities
- **Comprehensive Audit Logging**: All operations tracked with session context and client information
- **Advanced Rate Limiting**: Multi-tier rate limiting with burst protection and per-resource controls
- **Automatic Key Rotation**: Scheduled rotation with zero-downtime transitions and event coordination
- **Session Security**: Automatic token expiration, revocation lists, and distributed cache invalidation

### Project Context Management
- **Scoped Operations**: Focus AI operations on specific projects/repositories with security boundaries
- **Multi-Platform Support**: GitLab, GitHub, and custom contexts with unified authentication
- **Intelligent Caching**: Project structure and content cached in NATS KV stores for high performance
- **Resource Limits**: Fine-grained control over file sizes, search results, and operation scope
- **Combined Contexts**: Secure cross-project coordination with proper permission validation
- **Context Security**: Project-level access controls integrated with MCP authentication system

### Integration Capabilities
- **Multi-Protocol Support**: HTTP and WebSocket MCP transports
- **External API Access**: Secure access to GitLab, GitHub, and other services
- **Workflow Integration**: Seamless integration with Circuit Breaker workflows
- **Event-Driven Processing**: React to webhooks and external events
- **Context-Aware Operations**: AI agents work within defined project boundaries

### Developer Experience
- **Multiple SDKs**: Python, JavaScript/TypeScript, and more
- **Clear Documentation**: Comprehensive guides and examples
- **Easy Deployment**: Docker and Kubernetes ready
- **Monitoring Tools**: Built-in metrics and alerting
- **Project-Focused Workflows**: AI agents understand project structure and context

### Agent Swarm Coordination
- **Context-Aware Agents**: Agents operate within project boundaries for focused analysis
- **Cross-Project Intelligence**: Combined contexts enable multi-repo agent coordination
- **Efficient Resource Usage**: Project contexts prevent unnecessary broad searches
- **Scalable Architecture**: Handle multiple projects and teams simultaneously

This architecture positions Circuit Breaker as a secure, scalable platform for AI-powered automation that can safely interact with external services while maintaining strict security controls, project-focused operations, and comprehensive audit trails. The simplified storage architecture ensures both performance and security, with NATS JetStream providing real-time session management, high-performance caching, and token coordination through multiple KV stores, while PostgreSQL maintains persistent security credentials and audit compliance. The project context system ensures AI agents work efficiently within defined boundaries while enabling powerful cross-project coordination when needed, all backed by enterprise-grade authentication and session management using only PostgreSQL and NATS infrastructure.