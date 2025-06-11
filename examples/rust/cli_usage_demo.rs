//! CLI Usage Demo
//!
//! This example demonstrates how to use the MCP CLI programmatically
//! and shows common usage patterns for authentication and OAuth flows.

use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;
use tokio::time::{sleep, Duration};

/// CLI wrapper for easier programmatic usage
struct MCPCli {
    server_url: String,
    config_path: String,
    verbose: bool,
}

impl MCPCli {
    fn new(server_url: String, config_path: String) -> Self {
        Self {
            server_url,
            config_path,
            verbose: false,
        }
    }

    fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Execute a CLI command and return the output
    fn execute(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "--example", "mcp_cli", "--"])
            .arg("--server-url")
            .arg(&self.server_url)
            .arg("--config")
            .arg(&self.config_path);

        if self.verbose {
            cmd.arg("--verbose");
        }

        cmd.args(args);

        let output = cmd.output().context("Failed to execute CLI command")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("CLI command failed: {}", error);
        }
    }

    /// Execute a CLI command and parse JSON response
    fn execute_json(&self, args: &[&str]) -> Result<Value> {
        let output = self.execute(args)?;
        serde_json::from_str(&output).context("Failed to parse JSON response")
    }

    /// Check server status
    fn server_status(&self) -> Result<Value> {
        self.execute_json(&["server", "status"])
    }

    /// Create an app
    fn create_app(&self, name: &str, description: Option<&str>) -> Result<Value> {
        let mut args = vec!["auth", "create-app", "--name", name];
        if let Some(desc) = description {
            args.extend(&["--description", desc]);
        }
        self.execute_json(&args)
    }

    /// Install an app
    fn install_app(&self, app_id: &str, context: Option<&str>) -> Result<Value> {
        let mut args = vec!["auth", "install", "--app-id", app_id];
        if let Some(ctx) = context {
            args.extend(&["--context", ctx]);
        }
        self.execute_json(&args)
    }

    /// Login with app credentials
    fn login(&self, app_id: &str, installation_id: Option<&str>) -> Result<Value> {
        let mut args = vec!["auth", "login", "--app-id", app_id];
        if let Some(inst_id) = installation_id {
            args.extend(&["--installation-id", inst_id]);
        }
        self.execute_json(&args)
    }

    /// Get authentication status
    fn auth_status(&self) -> Result<String> {
        self.execute(&["auth", "status"])
    }

    /// Register OAuth provider
    fn register_oauth_provider(
        &self,
        provider: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        scopes: Option<&str>,
    ) -> Result<String> {
        let mut args = vec![
            "oauth",
            "register",
            "--provider",
            provider,
            "--client-id",
            client_id,
            "--client-secret",
            client_secret,
            "--redirect-uri",
            redirect_uri,
        ];
        if let Some(scopes) = scopes {
            args.extend(&["--scopes", scopes]);
        }
        self.execute(&args)
    }

    /// Start OAuth authorization
    fn start_oauth_auth(&self, provider: &str, installation_id: &str) -> Result<Value> {
        self.execute_json(&[
            "oauth",
            "authorize",
            "--provider",
            provider,
            "--installation-id",
            installation_id,
        ])
    }

    /// Complete OAuth callback
    fn complete_oauth_callback(&self, code: &str, state: &str) -> Result<Value> {
        self.execute_json(&["oauth", "callback", "--code", code, "--state", state])
    }

    /// List OAuth tokens
    fn list_oauth_tokens(&self) -> Result<String> {
        self.execute(&["oauth", "list"])
    }

    /// List sessions
    fn list_sessions(&self) -> Result<String> {
        self.execute(&["session", "list"])
    }

    /// Logout
    fn logout(&self) -> Result<String> {
        self.execute(&["auth", "logout"])
    }
}

/// Demo of complete authentication flow
async fn demo_auth_flow() -> Result<()> {
    println!("🚀 Starting MCP CLI Authentication Flow Demo");

    let cli = MCPCli::new(
        "http://localhost:3000".to_string(),
        "/tmp/mcp-cli-demo.json".to_string(),
    )
    .verbose(true);

    // Step 1: Check server status
    println!("\n📡 Step 1: Checking server status...");
    match cli.server_status() {
        Ok(status) => {
            println!(
                "✅ Server is running: {}",
                serde_json::to_string_pretty(&status)?
            );
        }
        Err(e) => {
            println!("❌ Server check failed: {}", e);
            println!("Please start the server with: cargo run --bin server");
            return Ok(());
        }
    }

    // Step 2: Create application
    println!("\n🏗️ Step 2: Creating MCP application...");
    let app_name = "demo-cli-app";
    let app_description = "Demo application created via CLI";

    let app_response = cli.create_app(app_name, Some(app_description))?;
    println!(
        "✅ App created: {}",
        serde_json::to_string_pretty(&app_response)?
    );

    // Extract app_id (in real usage, you'd parse this properly)
    let app_id = app_response
        .get("app_id")
        .and_then(|id| id.as_str())
        .context("Failed to get app_id from response")?;

    // Step 3: Install application
    println!("\n📦 Step 3: Installing application...");
    let install_response = cli.install_app(app_id, Some("demo-context"))?;
    println!(
        "✅ App installed: {}",
        serde_json::to_string_pretty(&install_response)?
    );

    // Extract installation_id
    let installation_id = install_response
        .get("installation_id")
        .and_then(|id| id.as_str())
        .context("Failed to get installation_id from response")?;

    // Step 4: Login
    println!("\n🔐 Step 4: Logging in...");
    let login_response = cli.login(app_id, Some(installation_id))?;
    println!(
        "✅ Login successful: {}",
        serde_json::to_string_pretty(&login_response)?
    );

    // Step 5: Check authentication status
    println!("\n📊 Step 5: Checking authentication status...");
    let auth_status = cli.auth_status()?;
    println!("✅ Auth status:\n{}", auth_status);

    // Step 6: List sessions
    println!("\n📋 Step 6: Listing sessions...");
    let sessions = cli.list_sessions()?;
    println!("✅ Sessions:\n{}", sessions);

    println!("\n🎉 Authentication flow demo completed successfully!");
    Ok(())
}

/// Demo of OAuth provider integration
async fn demo_oauth_flow() -> Result<()> {
    println!("🔗 Starting OAuth Integration Demo");

    let cli = MCPCli::new(
        "http://localhost:3000".to_string(),
        "/tmp/mcp-cli-demo.json".to_string(),
    )
    .verbose(true);

    // This is a demo - in real usage you'd have actual OAuth credentials
    let demo_credentials = [
        (
            "gitlab",
            "demo_client_id",
            "demo_client_secret",
            "read_user,read_repository",
        ),
        (
            "github",
            "demo_client_id",
            "demo_client_secret",
            "user:read,repo:read",
        ),
        (
            "google",
            "demo_client_id",
            "demo_client_secret",
            "openid,profile,email",
        ),
    ];

    for (provider, client_id, client_secret, scopes) in &demo_credentials {
        println!("\n🔧 Registering {} OAuth provider...", provider);

        match cli.register_oauth_provider(
            provider,
            client_id,
            client_secret,
            "http://localhost:3000/callback",
            Some(scopes),
        ) {
            Ok(response) => {
                println!("✅ {} provider registered: {}", provider, response);
            }
            Err(e) => {
                println!("⚠️ Failed to register {} provider: {}", provider, e);
            }
        }
    }

    // List OAuth tokens
    println!("\n📋 Listing OAuth tokens...");
    let tokens = cli.list_oauth_tokens()?;
    println!("OAuth tokens:\n{}", tokens);

    println!("\n🎉 OAuth integration demo completed!");
    Ok(())
}

/// Demo of session management
async fn demo_session_management() -> Result<()> {
    println!("📱 Starting Session Management Demo");

    let cli = MCPCli::new(
        "http://localhost:3000".to_string(),
        "/tmp/mcp-cli-demo.json".to_string(),
    )
    .verbose(true);

    // Create multiple sessions for demo
    println!("\n🔄 Creating multiple sessions...");

    for i in 1..=3 {
        let app_name = format!("session-demo-app-{}", i);
        let app_response = cli.create_app(&app_name, Some(&format!("Demo app {}", i)))?;

        if let Some(app_id) = app_response.get("app_id").and_then(|id| id.as_str()) {
            let install_response = cli.install_app(app_id, Some(&format!("context-{}", i)))?;

            if let Some(installation_id) = install_response
                .get("installation_id")
                .and_then(|id| id.as_str())
            {
                let login_response = cli.login(app_id, Some(installation_id))?;
                println!("✅ Session {} created", i);

                // Small delay between sessions
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    // List all sessions
    println!("\n📋 Listing all sessions...");
    let sessions = cli.list_sessions()?;
    println!("All sessions:\n{}", sessions);

    println!("\n🎉 Session management demo completed!");
    Ok(())
}

/// Cleanup demo data
async fn cleanup_demo() -> Result<()> {
    println!("🧹 Cleaning up demo data...");

    let cli = MCPCli::new(
        "http://localhost:3000".to_string(),
        "/tmp/mcp-cli-demo.json".to_string(),
    );

    // Logout
    match cli.logout() {
        Ok(response) => println!("✅ Logged out: {}", response),
        Err(e) => println!("⚠️ Logout failed: {}", e),
    }

    // Remove demo config file
    if std::path::Path::new("/tmp/mcp-cli-demo.json").exists() {
        std::fs::remove_file("/tmp/mcp-cli-demo.json")?;
        println!("✅ Removed demo config file");
    }

    println!("🎉 Cleanup completed!");
    Ok(())
}

/// Interactive demo menu
async fn interactive_demo() -> Result<()> {
    println!("🎮 MCP CLI Interactive Demo");
    println!("==========================");

    loop {
        println!("\nSelect a demo:");
        println!("1. Authentication Flow");
        println!("2. OAuth Integration");
        println!("3. Session Management");
        println!("4. Run All Demos");
        println!("5. Cleanup Demo Data");
        println!("6. Exit");

        print!("Enter your choice (1-6): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                if let Err(e) = demo_auth_flow().await {
                    println!("❌ Demo failed: {}", e);
                }
            }
            "2" => {
                if let Err(e) = demo_oauth_flow().await {
                    println!("❌ Demo failed: {}", e);
                }
            }
            "3" => {
                if let Err(e) = demo_session_management().await {
                    println!("❌ Demo failed: {}", e);
                }
            }
            "4" => {
                println!("🚀 Running all demos...");
                if let Err(e) = demo_auth_flow().await {
                    println!("❌ Auth demo failed: {}", e);
                }
                if let Err(e) = demo_oauth_flow().await {
                    println!("❌ OAuth demo failed: {}", e);
                }
                if let Err(e) = demo_session_management().await {
                    println!("❌ Session demo failed: {}", e);
                }
                println!("🎉 All demos completed!");
            }
            "5" => {
                if let Err(e) = cleanup_demo().await {
                    println!("❌ Cleanup failed: {}", e);
                }
            }
            "6" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => {
                println!("❌ Invalid choice. Please enter 1-6.");
            }
        }
    }

    Ok(())
}

/// Automated demo with all features
async fn automated_demo() -> Result<()> {
    println!("🤖 Starting Automated MCP CLI Demo");
    println!("==================================");

    // Run all demos in sequence
    println!("\n1️⃣ Running Authentication Flow Demo...");
    demo_auth_flow().await?;

    println!("\n2️⃣ Running OAuth Integration Demo...");
    demo_oauth_flow().await?;

    println!("\n3️⃣ Running Session Management Demo...");
    demo_session_management().await?;

    println!("\n🧹 Running Cleanup...");
    cleanup_demo().await?;

    println!("\n🎉 Automated demo completed successfully!");
    println!("All MCP CLI features have been demonstrated.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("auth") => demo_auth_flow().await,
        Some("oauth") => demo_oauth_flow().await,
        Some("sessions") => demo_session_management().await,
        Some("cleanup") => cleanup_demo().await,
        Some("auto") => automated_demo().await,
        Some("interactive") | None => interactive_demo().await,
        Some(arg) => {
            println!("❌ Unknown argument: {}", arg);
            println!(
                "\nUsage: {} [auth|oauth|sessions|cleanup|auto|interactive]",
                args[0]
            );
            println!("\nOptions:");
            println!("  auth        - Run authentication flow demo");
            println!("  oauth       - Run OAuth integration demo");
            println!("  sessions    - Run session management demo");
            println!("  cleanup     - Clean up demo data");
            println!("  auto        - Run all demos automatically");
            println!("  interactive - Interactive demo menu (default)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_creation() {
        let cli = MCPCli::new(
            "http://localhost:3000".to_string(),
            "/tmp/test-config.json".to_string(),
        );

        assert_eq!(cli.server_url, "http://localhost:3000");
        assert_eq!(cli.config_path, "/tmp/test-config.json");
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_verbose() {
        let cli = MCPCli::new(
            "http://localhost:3000".to_string(),
            "/tmp/test-config.json".to_string(),
        )
        .verbose(true);

        assert!(cli.verbose);
    }
}
