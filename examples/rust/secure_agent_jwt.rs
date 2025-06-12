//! Secure Agent JWT Authentication Demo
//!
//! This example demonstrates the complete JWT authentication flow for MCP agents,
//! showing how to securely authenticate with the Circuit Breaker MCP server using
//! the GitHub Apps-inspired authentication model.
//!
//! ## Authentication Flow
//!
//! 1. **App Registration**: Create MCP app with RSA key pair
//! 2. **App Installation**: Install app to organization/user
//! 3. **JWT Generation**: Create short-lived app JWT using private key
//! 4. **Session Token**: Exchange app JWT for session access token
//! 5. **API Requests**: Use session token for MCP operations
//! 6. **Token Refresh**: Handle token expiration and renewal
//!
//! ## Usage
//!
//! ```bash
//! # Run the complete JWT authentication demo
//! cargo run --example secure_agent_jwt demo full
//!
//! # Test JWT generation only
//! cargo run --example secure_agent_jwt demo jwt-only
//!
//! # Test session management
//! cargo run --example secure_agent_jwt demo session-mgmt
//!
//! # Interactive mode
//! cargo run --example secure_agent_jwt interactive
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey, DecodingKey, Validation};
use reqwest::Client;
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs1::EncodeRsaPublicKey, pkcs8::EncodePrivateKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::{sleep, Duration as TokioDuration};
use uuid::Uuid;

/// CLI for JWT authentication demo
#[derive(Parser)]
#[command(name = "secure-agent-jwt")]
#[command(about = "Secure Agent JWT Authentication Demo")]
struct Cli {
    /// MCP Server URL
    #[arg(long, env = "MCP_SERVER_URL", default_value = "http://localhost:3000")]
    server_url: String,

    /// Configuration file path
    #[arg(long, default_value = "~/.secure-agent-jwt.json")]
    config: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run authentication demos
    Demo {
        #[command(subcommand)]
        action: DemoAction,
    },
    /// Interactive mode
    Interactive,
    /// Generate JWT tokens
    Jwt {
        #[command(subcommand)]
        action: JwtAction,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum DemoAction {
    /// Complete JWT authentication flow
    Full {
        /// Skip confirmation prompts
        #[arg(long)]
        auto_confirm: bool,
    },
    /// JWT generation and validation only
    JwtOnly,
    /// Session management demo
    SessionMgmt,
    /// Token refresh demo
    TokenRefresh,
}

#[derive(Subcommand)]
enum JwtAction {
    /// Generate app JWT
    Generate {
        /// App ID
        #[arg(short, long)]
        app_id: String,
        /// Private key file path
        #[arg(short, long)]
        private_key: String,
    },
    /// Validate JWT token
    Validate {
        /// JWT token to validate
        #[arg(short, long)]
        token: String,
        /// Public key file path
        #[arg(short, long)]
        public_key: String,
    },
    /// Generate RSA key pair
    GenerateKeys {
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output_dir: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Create session token
    Create {
        /// App JWT token
        #[arg(short, long)]
        app_jwt: String,
        /// Installation ID
        #[arg(short, long)]
        installation_id: String,
    },
    /// List active sessions
    List,
    /// Validate session token
    Validate {
        /// Session token
        #[arg(short, long)]
        token: String,
    },
    /// Revoke session
    Revoke {
        /// Session token
        #[arg(short, long)]
        token: String,
    },
}

/// Configuration for the JWT demo
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtConfig {
    server_url: String,
    current_app: Option<AppInfo>,
    current_session: Option<SessionInfo>,
    apps: HashMap<String, AppInfo>,
    sessions: HashMap<String, SessionInfo>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_string(),
            current_app: None,
            current_session: None,
            apps: HashMap::new(),
            sessions: HashMap::new(),
        }
    }
}

/// MCP App information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppInfo {
    app_id: String,
    name: String,
    description: String,
    private_key: String,
    public_key: String,
    client_id: String,
    client_secret: String,
    created_at: DateTime<Utc>,
    installations: Vec<InstallationInfo>,
}

/// Installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallationInfo {
    installation_id: String,
    app_id: String,
    account_type: String,
    permissions: Value,
    created_at: DateTime<Utc>,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionInfo {
    session_id: String,
    app_id: String,
    installation_id: String,
    access_token: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    permissions: Value,
}

/// JWT Claims for MCP App authentication
#[derive(Debug, Serialize, Deserialize)]
struct AppJwtClaims {
    iss: String, // App ID (issuer)
    iat: i64,    // Issued at
    exp: i64,    // Expires at
    aud: String, // Audience - "circuit-breaker-mcp"
}

/// JWT Claims for session tokens
#[derive(Debug, Serialize, Deserialize)]
struct SessionJwtClaims {
    iss: String, // "circuit-breaker-mcp"
    sub: String, // Installation ID
    app_id: String,
    installation_id: String,
    permissions: Value,
    iat: i64,    // Issued at
    exp: i64,    // Expires at
    jti: String, // JWT ID (unique token identifier)
}

/// Main application struct
struct JwtDemo {
    config: JwtConfig,
    config_path: PathBuf,
    client: Client,
    verbose: bool,
}

impl JwtDemo {
    fn new(server_url: String, config_path: String, verbose: bool) -> Result<Self> {
        let config_path = PathBuf::from(shellexpand::tilde(&config_path).to_string());
        
        let config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            serde_json::from_str(&config_str)
                .context("Failed to parse config file")?
        } else {
            JwtConfig {
                server_url: server_url.clone(),
                ..Default::default()
            }
        };

        Ok(Self {
            config,
            config_path,
            client: Client::new(),
            verbose,
        })
    }

    fn save_config(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let config_str = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, config_str)?;
        
        if self.verbose {
            println!("💾 Config saved to: {}", self.config_path.display());
        }
        
        Ok(())
    }

    /// Generate RSA key pair for app authentication
    fn generate_rsa_keys(&self) -> Result<(String, String)> {
        use rand::rngs::OsRng;
        
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits)
            .context("Failed to generate RSA private key")?;
        let public_key = RsaPublicKey::from(&private_key);

        let private_pem = private_key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .context("Failed to encode private key")?;
        let public_pem = public_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .context("Failed to encode public key")?;

        Ok((private_pem.to_string(), public_pem))
    }

    /// Generate App JWT for authentication
    fn generate_app_jwt(&self, app_id: &str, private_key_pem: &str) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::minutes(10); // 10-minute expiry for app JWTs

        let claims = AppJwtClaims {
            iss: app_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            aud: "circuit-breaker-mcp".to_string(),
        };

        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .context("Failed to create encoding key from private key")?;

        let token = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
            .context("Failed to encode JWT")?;

        Ok(token)
    }

    /// Validate App JWT
    fn validate_app_jwt(&self, token: &str, public_key_pem: &str) -> Result<AppJwtClaims> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .context("Failed to create decoding key from public key")?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&["circuit-breaker-mcp"]);

        let token_data = decode::<AppJwtClaims>(token, &decoding_key, &validation)
            .context("Failed to decode JWT")?;

        Ok(token_data.claims)
    }

    /// Make HTTP request to MCP server
    async fn make_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        auth_token: Option<&str>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.config.server_url, path);
        
        let mut request = self.client.request(method, &url);
        
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        if let Some(body) = body {
            request = request.json(&body);
        }

        if self.verbose {
            println!("🌐 Making request to: {}", url);
        }

        let response = request.send().await
            .context("Failed to send HTTP request")?;

        Ok(response)
    }

    /// Create MCP app
    async fn create_app(&mut self, name: &str, description: &str) -> Result<AppInfo> {
        println!("🏗️ Creating MCP app: {}", name);

        // Generate RSA key pair
        let (private_key, public_key) = self.generate_rsa_keys()?;

        // Create app request
        let app_request = json!({
            "name": name,
            "description": description,
            "permissions": {
                "workflows": "write",
                "agents": "write",
                "functions": "read",
                "external_apis": {
                    "gitlab": {
                        "scopes": ["api", "read_user"],
                        "allowed_endpoints": ["/api/v4/projects/*"]
                    }
                }
            }
        });

        let response = self.make_request(
            reqwest::Method::POST,
            "/api/v1/mcp/apps",
            Some(app_request),
            None,
        ).await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to create app: {}", error_text);
        }

        let app_data: Value = response.json().await?;
        
        let app_info = AppInfo {
            app_id: app_data["app_id"].as_str().unwrap().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            private_key,
            public_key,
            client_id: app_data["client_id"].as_str().unwrap().to_string(),
            client_secret: app_data["client_secret"].as_str().unwrap().to_string(),
            created_at: Utc::now(),
            installations: Vec::new(),
        };

        // Save to config
        self.config.apps.insert(app_info.app_id.clone(), app_info.clone());
        self.config.current_app = Some(app_info.clone());
        self.save_config()?;

        println!("✅ App created successfully!");
        println!("   App ID: {}", app_info.app_id);
        println!("   Client ID: {}", app_info.client_id);

        Ok(app_info)
    }

    /// Install app to organization/user
    async fn install_app(&mut self, app_id: &str) -> Result<InstallationInfo> {
        println!("📦 Installing app: {}", app_id);

        let app = self.config.apps.get(app_id)
            .context("App not found in config")?;

        // Generate app JWT for installation
        let app_jwt = self.generate_app_jwt(&app.app_id, &app.private_key)?;

        let install_request = json!({
            "app_id": app_id,
            "account_type": "user",
            "permissions": {
                "workflows": "write",
                "agents": "write"
            }
        });

        let response = self.make_request(
            reqwest::Method::POST,
            &format!("/api/v1/mcp/apps/{}/installations", app_id),
            Some(install_request),
            Some(&app_jwt),
        ).await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to install app: {}", error_text);
        }

        let install_data: Value = response.json().await?;
        
        let installation_info = InstallationInfo {
            installation_id: install_data["installation_id"].as_str().unwrap().to_string(),
            app_id: app_id.to_string(),
            account_type: "user".to_string(),
            permissions: install_data["permissions"].clone(),
            created_at: Utc::now(),
        };

        // Update app with installation
        if let Some(app) = self.config.apps.get_mut(app_id) {
            app.installations.push(installation_info.clone());
        }
        self.save_config()?;

        println!("✅ App installed successfully!");
        println!("   Installation ID: {}", installation_info.installation_id);

        Ok(installation_info)
    }

    /// Create session token from app JWT
    async fn create_session_token(
        &mut self,
        app_id: &str,
        installation_id: &str,
    ) -> Result<SessionInfo> {
        println!("🔐 Creating session token...");

        let app = self.config.apps.get(app_id)
            .context("App not found in config")?;

        // Generate app JWT
        let app_jwt = self.generate_app_jwt(&app.app_id, &app.private_key)?;

        let token_request = json!({
            "installation_id": installation_id,
            "permissions": {
                "workflows": "write",
                "agents": "write"
            }
        });

        let response = self.make_request(
            reqwest::Method::POST,
            &format!("/api/v1/mcp/apps/{}/installations/{}/access_tokens", app_id, installation_id),
            Some(token_request),
            Some(&app_jwt),
        ).await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to create session token: {}", error_text);
        }

        let token_data: Value = response.json().await?;
        
        let session_info = SessionInfo {
            session_id: Uuid::new_v4().to_string(),
            app_id: app_id.to_string(),
            installation_id: installation_id.to_string(),
            access_token: token_data["token"].as_str().unwrap().to_string(),
            expires_at: DateTime::parse_from_rfc3339(
                token_data["expires_at"].as_str().unwrap()
            )?.with_timezone(&Utc),
            created_at: Utc::now(),
            permissions: token_data["permissions"].clone(),
        };

        // Save session
        self.config.sessions.insert(session_info.session_id.clone(), session_info.clone());
        self.config.current_session = Some(session_info.clone());
        self.save_config()?;

        println!("✅ Session token created successfully!");
        println!("   Session ID: {}", session_info.session_id);
        println!("   Expires at: {}", session_info.expires_at);

        Ok(session_info)
    }

    /// Test MCP operations with session token
    async fn test_mcp_operations(&self, session_token: &str) -> Result<()> {
        println!("🧪 Testing MCP operations...");

        // Test 1: List available tools
        println!("\n1️⃣ Testing list_tools...");
        let list_tools_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        });

        let response = self.make_request(
            reqwest::Method::POST,
            "/mcp/v1/transport/http",
            Some(list_tools_request),
            Some(session_token),
        ).await?;

        if response.status().is_success() {
            let tools_data: Value = response.json().await?;
            println!("✅ Available tools: {}", serde_json::to_string_pretty(&tools_data)?);
        } else {
            println!("❌ Failed to list tools: {}", response.status());
        }

        // Test 2: Call a tool (if available)
        println!("\n2️⃣ Testing tool call...");
        let call_tool_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "Hello from secure JWT demo!"
                }
            }
        });

        let response = self.make_request(
            reqwest::Method::POST,
            "/mcp/v1/transport/http",
            Some(call_tool_request),
            Some(session_token),
        ).await?;

        if response.status().is_success() {
            let result: Value = response.json().await?;
            println!("✅ Tool call result: {}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("⚠️ Tool call failed (may not be implemented): {}", response.status());
        }

        // Test 3: List resources
        println!("\n3️⃣ Testing list_resources...");
        let list_resources_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/list"
        });

        let response = self.make_request(
            reqwest::Method::POST,
            "/mcp/v1/transport/http",
            Some(list_resources_request),
            Some(session_token),
        ).await?;

        if response.status().is_success() {
            let resources_data: Value = response.json().await?;
            println!("✅ Available resources: {}", serde_json::to_string_pretty(&resources_data)?);
        } else {
            println!("⚠️ Failed to list resources: {}", response.status());
        }

        println!("\n🎉 MCP operations test completed!");
        Ok(())
    }

    /// Demo breakpoint with user confirmation
    async fn demo_breakpoint(
        &self,
        step: u32,
        title: &str,
        description: &str,
        auto_confirm: bool,
    ) -> Result<()> {
        println!("\n{}", "=".repeat(60));
        println!("🔹 Step {}: {}", step, title);
        println!("{}", description);
        println!("{}", "=".repeat(60));

        if !auto_confirm {
            print!("\nPress Enter to continue...");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
        } else {
            sleep(TokioDuration::from_millis(1000)).await;
        }

        Ok(())
    }

    /// Run complete JWT authentication demo
    async fn run_full_demo(&mut self, auto_confirm: bool) -> Result<()> {
        println!("🚀 Starting Complete JWT Authentication Demo");
        println!("============================================");

        // Step 1: Environment Setup
        self.demo_breakpoint(
            1,
            "Environment Setup and Validation",
            "Setting up the demo environment and validating server connectivity.\n\
             This step ensures the MCP server is running and accessible.",
            auto_confirm,
        ).await?;

        // Check server status
        match self.make_request(reqwest::Method::GET, "/health", None, None).await {
            Ok(response) if response.status().is_success() => {
                println!("✅ MCP Server is running at: {}", self.config.server_url);
            }
            _ => {
                println!("❌ MCP Server is not accessible at: {}", self.config.server_url);
                println!("Please start the server and try again.");
                return Ok(());
            }
        }

        // Step 2: App Creation
        self.demo_breakpoint(
            2,
            "MCP App Creation",
            "Creating a new MCP application with RSA key pair generation.\n\
             This demonstrates the GitHub Apps-inspired registration process.",
            auto_confirm,
        ).await?;

        let app = self.create_app(
            "JWT Demo App",
            "Demonstration app for JWT authentication flow"
        ).await?;

        // Step 3: App Installation
        self.demo_breakpoint(
            3,
            "App Installation",
            "Installing the MCP app to enable authentication.\n\
             This creates the installation context for JWT token exchange.",
            auto_confirm,
        ).await?;

        let installation = self.install_app(&app.app_id).await?;

        // Step 4: JWT Generation
        self.demo_breakpoint(
            4,
            "App JWT Generation",
            "Generating short-lived App JWT using RSA private key.\n\
             This JWT will be used to authenticate with the MCP server.",
            auto_confirm,
        ).await?;

        let app_jwt = self.generate_app_jwt(&app.app_id, &app.private_key)?;
        println!("✅ App JWT generated (10-minute expiry)");
        if self.verbose {
            println!("   JWT: {}...", &app_jwt[..50]);
        }

        // Step 5: Session Token Creation
        self.demo_breakpoint(
            5,
            "Session Token Creation",
            "Exchanging App JWT for session access token.\n\
             This creates a longer-lived token for MCP operations.",
            auto_confirm,
        ).await?;

        let session = self.create_session_token(&app.app_id, &installation.installation_id).await?;

        // Step 6: MCP Operations
        self.demo_breakpoint(
            6,
            "MCP Operations Testing",
            "Testing MCP operations using the session token.\n\
             This demonstrates authenticated API calls to the MCP server.",
            auto_confirm,
        ).await?;

        self.test_mcp_operations(&session.access_token).await?;

        // Step 7: Token Validation
        self.demo_breakpoint(
            7,
            "Token Validation Demo",
            "Demonstrating JWT validation and claims inspection.\n\
             This shows how to verify token authenticity and extract claims.",
            auto_confirm,
        ).await?;

        // Validate the app JWT we generated
        match self.validate_app_jwt(&app_jwt, &app.public_key) {
            Ok(claims) => {
                println!("✅ App JWT validation successful!");
                println!("   Issuer (App ID): {}", claims.iss);
                println!("   Audience: {}", claims.aud);
                println!("   Issued at: {}", DateTime::from_timestamp(claims.iat, 0).unwrap());
                println!("   Expires at: {}", DateTime::from_timestamp(claims.exp, 0).unwrap());
            }
            Err(e) => {
                println!("❌ JWT validation failed: {}", e);
            }
        }

        // Step 8: Session Management
        self.demo_breakpoint(
            8,
            "Session Management",
            "Demonstrating session listing and management capabilities.\n\
             This shows how to track and manage active authentication sessions.",
            auto_confirm,
        ).await?;

        self.list_sessions().await?;

        println!("\n🎉 Complete JWT Authentication Demo Finished!");
        println!("===============================================");
        println!("✅ Successfully demonstrated:");
        println!("   • MCP App creation with RSA key generation");
        println!("   • App installation and permission setup");
        println!("   • JWT generation and validation");
        println!("   • Session token creation and management");
        println!("   • Authenticated MCP operations");
        println!("   • Token validation and claims inspection");

        Ok(())
    }

    /// Run JWT-only demo
    async fn run_jwt_only_demo(&mut self) -> Result<()> {
        println!("🔐 JWT Generation and Validation Demo");
        println!("====================================");

        // Generate RSA keys
        println!("\n1️⃣ Generating RSA key pair...");
        let (private_key, public_key) = self.generate_rsa_keys()?;
        println!("✅ RSA key pair generated (2048 bits)");

        // Generate App JWT
        println!("\n2️⃣ Generating App JWT...");
        let app_id = format!("demo-app-{}", Uuid::new_v4());
        let app_jwt = self.generate_app_jwt(&app_id, &private_key)?;
        println!("✅ App JWT generated");
        println!("   App ID: {}", app_id);
        println!("   JWT: {}...", &app_jwt[..50]);

        // Validate JWT
        println!("\n3️⃣ Validating App JWT...");
        match self.validate_app_jwt(&app_jwt, &public_key) {
            Ok(claims) => {
                println!("✅ JWT validation successful!");
                println!("   Issuer: {}", claims.iss);
                println!("   Audience: {}", claims.aud);
                println!("   Issued at: {}", DateTime::from_timestamp(claims.iat, 0).unwrap());
                println!("   Expires at: {}", DateTime::from_timestamp(claims.exp, 0).unwrap());
            }
            Err(e) => {
                println!("❌ JWT validation failed: {}", e);
            }
        }

        // Test with invalid JWT
        println!("\n4️⃣ Testing invalid JWT...");
        let invalid_jwt = "invalid.jwt.token";
        match self.validate_app_jwt(invalid_jwt, &public_key) {
            Ok(_) => println!("❌ Unexpected: Invalid JWT was accepted!"),
            Err(_) => println!("✅ Invalid JWT correctly rejected"),
        }

        println!("\n🎉 JWT Demo completed!");
        Ok(())
    }

    /// Run session management demo
    async fn run_session_mgmt_demo(&mut self) -> Result<()> {
        println!("📱 Session Management Demo");
        println!("=========================");

        // Create multiple demo sessions
        println!("\n1️⃣ Creating demo sessions...");
        
        for i in 1..=3 {
            let app_name = format!("Session Demo App {}", i);
            let app = self.create_app(&app_name, &format!("Demo app for session {}", i)).await?;
            let installation = self.install_app(&app.app_id).await?;
            let _session = self.create_session_token(&app.app_id, &installation.installation_id).await?;
            
            println!("✅ Session {} created", i);
        }

        // List all sessions
        println!("\n2️⃣ Listing all sessions...");
        self.list_sessions().await?;

        // Test session validation
        println!("\n3️⃣ Testing session validation...");
        if let Some(session) = self.config.current_session.as_ref() {
            println!("Testing current session: {}", session.session_id);
            
            // Check if session is expired
            let now = Utc::now();
            if session.expires_at > now {
                println!("✅ Session is valid (expires in {} minutes)", 
                    (session.expires_at - now).num_minutes());
            } else {
                println!("❌ Session has expired");
            }
        }

        println!("\n🎉 Session management demo completed!");
        Ok(())
    }

    /// List active sessions
    async fn list_sessions(&self) -> Result<()> {
        println!("📋 Active Sessions:");
        println!("==================");

        if self.config.sessions.is_empty() {
            println!("No active sessions found.");
            return Ok(());
        }

        for (session_id, session) in &self.config.sessions {
            let status = if session.expires_at > Utc::now() {
                "✅ Active"
            } else {
                "❌ Expired"
            };

            println!("Session ID: {}", session_id);
            println!("  App ID: {}", session.app_id);
            println!("  Installation ID: {}", session.installation_id);
            println!("  Status: {}", status);
            println!("  Created: {}", session.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("  Expires: {}", session.expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!();
        }

        Ok(())
    }

    /// Interactive mode
    async fn run_interactive(&mut self) -> Result<()> {
        println!("🎮 Secure Agent JWT Interactive Demo");
        println!("====================================");

        loop {
            println!("\nSelect an option:");
            println!("1. Complete JWT Authentication Flow");
            println!("2. JWT Generation and Validation Only");
            println!("3. Session Management Demo");
            println!("4. Token Refresh Demo");
            println!("5. List Current Sessions");
            println!("6. Generate RSA Key Pair");
            println!("7. Exit");

            print!("\nEnter your choice (1-7): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim() {
                "1" => {
                    if let Err(e) = self.run_full_demo(false).await {
                        println!("❌ Demo failed: {}", e);
                    }
                }
                "2" => {
                    if let Err(e) = self.run_jwt_only_demo().await {
                        println!("❌ JWT demo failed: {}", e);
                    }
                }
                "3" => {
                    if let Err(e) = self.run_session_mgmt_demo().await {
                        println!("❌ Session demo failed: {}", e);
                    }
                }
                "4" => {
                    if let Err(e) = self.run_token_refresh_demo().await {
                        println!("❌ Token refresh demo failed: {}", e);
                    }
                }
                "5" => {
                    if let Err(e) = self.list_sessions().await {
                        println!("❌ Failed to list sessions: {}", e);
                    }
                }
                "6" => {
                    if let Err(e) = self.generate_and_save_keys().await {
                        println!("❌ Key generation failed: {}", e);
                    }
                }
                "7" => {
                    println!("👋 Goodbye!");
                    break;
                }
                _ => {
                    println!("❌ Invalid choice. Please enter 1-7.");
                }
            }
        }

        Ok(())
    }

    /// Token refresh demo
    async fn run_token_refresh_demo(&mut self) -> Result<()> {
        println!("🔄 Token Refresh Demo");
        println!("====================");

        // Create a session if none exists
        if self.config.current_session.is_none() {
            println!("No active session found. Creating one...");
            let app = self.create_app("Refresh Demo App", "App for token refresh demo").await?;
            let installation = self.install_app(&app.app_id).await?;
            let _session = self.create_session_token(&app.app_id, &installation.installation_id).await?;
        }

        if let Some(session) = &self.config.current_session {
            println!("Current session expires at: {}", session.expires_at);
            
            let now = Utc::now();
            let time_until_expiry = session.expires_at - now;
            let app_id = session.app_id.clone();
            let installation_id = session.installation_id.clone();
            
            if time_until_expiry.num_minutes() > 5 {
                println!("Session is still valid for {} minutes", time_until_expiry.num_minutes());
                println!("In a real application, you would refresh the token before it expires.");
                
                // Simulate token refresh
                println!("\n🔄 Simulating token refresh...");
                let new_session = self.create_session_token(&app_id, &installation_id).await?;
                println!("✅ New session token created");
                println!("   New expiry: {}", new_session.expires_at);
            } else {
                println!("Session is close to expiry or expired. Refreshing...");
                let new_session = self.create_session_token(&app_id, &installation_id).await?;
                println!("✅ Session refreshed successfully");
                println!("   New expiry: {}", new_session.expires_at);
            }
        }

        Ok(())
    }

    /// Generate and save RSA keys to files
    async fn generate_and_save_keys(&self) -> Result<()> {
        println!("🔑 Generating RSA Key Pair");
        println!("==========================");

        let (private_key, public_key) = self.generate_rsa_keys()?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let private_key_path = format!("mcp_private_key_{}.pem", timestamp);
        let public_key_path = format!("mcp_public_key_{}.pem", timestamp);

        std::fs::write(&private_key_path, &private_key)?;
        std::fs::write(&public_key_path, &public_key)?;

        println!("✅ RSA key pair generated and saved:");
        println!("   Private key: {}", private_key_path);
        println!("   Public key: {}", public_key_path);
        println!("\n⚠️  Keep the private key secure and never share it!");

        Ok(())
    }

    /// Handle JWT commands
    async fn handle_jwt_action(&mut self, action: JwtAction) -> Result<()> {
        match action {
            JwtAction::Generate { app_id, private_key } => {
                let private_key_content = std::fs::read_to_string(&private_key)
                    .context("Failed to read private key file")?;
                
                let jwt = self.generate_app_jwt(&app_id, &private_key_content)?;
                println!("Generated JWT: {}", jwt);
            }
            JwtAction::Validate { token, public_key } => {
                let public_key_content = std::fs::read_to_string(&public_key)
                    .context("Failed to read public key file")?;
                
                match self.validate_app_jwt(&token, &public_key_content) {
                    Ok(claims) => {
                        println!("✅ JWT is valid");
                        println!("Claims: {}", serde_json::to_string_pretty(&claims)?);
                    }
                    Err(e) => {
                        println!("❌ JWT validation failed: {}", e);
                    }
                }
            }
            JwtAction::GenerateKeys { output_dir } => {
                let (private_key, public_key) = self.generate_rsa_keys()?;
                
                let private_path = format!("{}/private_key.pem", output_dir);
                let public_path = format!("{}/public_key.pem", output_dir);
                
                std::fs::write(&private_path, &private_key)?;
                std::fs::write(&public_path, &public_key)?;
                
                println!("✅ Keys generated:");
                println!("   Private: {}", private_path);
                println!("   Public: {}", public_path);
            }
        }
        Ok(())
    }

    /// Handle session commands
    async fn handle_session_action(&mut self, action: SessionAction) -> Result<()> {
        match action {
            SessionAction::Create { app_jwt, installation_id } => {
                // This would require parsing the JWT to get the app_id
                println!("Session creation via CLI not fully implemented");
                println!("Use the demo mode instead: cargo run --example secure_agent_jwt demo full");
            }
            SessionAction::List => {
                self.list_sessions().await?;
            }
            SessionAction::Validate { token: _token } => {
                println!("Session validation via CLI not implemented");
                println!("This would require server-side validation");
            }
            SessionAction::Revoke { token: _token } => {
                println!("Session revocation via CLI not implemented");
                println!("This would require server-side revocation");
            }
        }
        Ok(())
    }

    /// Run the application
    async fn run(mut self, command: Commands) -> Result<()> {
        match command {
            Commands::Demo { action } => {
                match action {
                    DemoAction::Full { auto_confirm } => {
                        self.run_full_demo(auto_confirm).await?;
                    }
                    DemoAction::JwtOnly => {
                        self.run_jwt_only_demo().await?;
                    }
                    DemoAction::SessionMgmt => {
                        self.run_session_mgmt_demo().await?;
                    }
                    DemoAction::TokenRefresh => {
                        self.run_token_refresh_demo().await?;
                    }
                }
            }
            Commands::Interactive => {
                self.run_interactive().await?;
            }
            Commands::Jwt { action } => {
                self.handle_jwt_action(action).await?;
            }
            Commands::Session { action } => {
                self.handle_session_action(action).await?;
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let app = JwtDemo::new(cli.server_url, cli.config, cli.verbose)?;
    app.run(cli.command).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_config_serialization() {
        let config = JwtConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: JwtConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.server_url, deserialized.server_url);
    }

    #[test]
    fn test_rsa_key_generation() {
        let demo = JwtDemo::new(
            "http://localhost:3000".to_string(),
            "/tmp/test-config.json".to_string(),
            false,
        ).unwrap();

        let (private_key, public_key) = demo.generate_rsa_keys().unwrap();
        assert!(private_key.contains("BEGIN PRIVATE KEY"));
        assert!(public_key.contains("BEGIN RSA PUBLIC KEY"));
    }

    #[test]
    fn test_jwt_generation_and_validation() {
        let demo = JwtDemo::new(
            "http://localhost:3000".to_string(),
            "/tmp/test-config.json".to_string(),
            false,
        ).unwrap();

        let (private_key, public_key) = demo.generate_rsa_keys().unwrap();
        let app_id = "test-app-123";
        
        let jwt = demo.generate_app_jwt(app_id, &private_key).unwrap();
        let claims = demo.validate_app_jwt(&jwt, &public_key).unwrap();
        
        assert_eq!(claims.iss, app_id);
        assert_eq!(claims.aud, "circuit-breaker-mcp");
    }
} 