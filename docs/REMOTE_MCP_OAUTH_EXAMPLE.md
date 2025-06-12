# MCP CLI - Complete Multi-Context Protocol Demo

This directory contains a comprehensive demonstration of the Multi-Context Protocol (MCP) server with GitLab integration. The `mcp_cli.rs` example provides a complete workflow that walks through setting up a multi-tenant remote MCP server, OAuth authentication, and GitLab API integration.

## Overview

The MCP CLI demonstrates:

1. **Server Health Check** - Verify MCP server is running and accessible
2. **MCP App Creation** - Create and register a new MCP application
3. **App Installation** - Install the app with project context detection
4. **OAuth Provider Registration** - Register GitLab as an OAuth provider
5. **Authentication Token Creation** - Generate installation tokens for API access
6. **OAuth Authorization Flow** - Browser-based GitLab authentication
7. **GitLab API Integration** - Test user information and project data retrieval
8. **Project Context Discovery** - Discover and display available projects
9. **Issue Management** - Demonstrate issue listing and management capabilities

## Prerequisites

### 1. MCP Server Running

Start the MCP server:
```bash
cargo run --bin server
```

### 2. NgRok Tunnel

Create a public tunnel to your local server:
```bash
ngrok http 3000
```

Note the NgRok URL (e.g., `https://abc123.ngrok-free.app`)

### 3. GitLab OAuth Application

Create a GitLab OAuth application:

1. Go to GitLab → Settings → Applications
2. Create new application with these settings:
   - **Name**: MCP Demo Application
   - **Redirect URI**: `{NGROK_URL}/mcp/oauth/callback`
   - **Scopes**: `api`, `read_user`, `read_repository`
3. Note the Client ID and Client Secret

### 4. Test Keys

Ensure you have RSA test keys in the `test_keys/` directory:
```bash
# Generate test keys if they don't exist
mkdir -p test_keys
openssl genrsa -out test_keys/test.pem 2048
openssl rsa -in test_keys/test.pem -pubout -out test_keys/test.pub
```

## Usage

### Complete Demo Workflow

Run the full demo with interactive breakpoints:

```bash
cargo run --example mcp_cli demo full \
  --ngrok-url https://your-ngrok-url.ngrok-free.app \
  --gitlab-client-id your_gitlab_client_id \
  --gitlab-client-secret your_gitlab_client_secret
```

Or with environment variables:
```bash
export NGROK_URL="https://your-ngrok-url.ngrok-free.app"
export GITLAB_CLIENT_ID="your_gitlab_client_id"
export GITLAB_CLIENT_SECRET="your_gitlab_client_secret"

cargo run --example mcp_cli demo full
```

### Auto-Confirm Mode

Skip interactive confirmations:
```bash
cargo run --example mcp_cli demo full --auto-confirm \
  --ngrok-url https://your-ngrok-url.ngrok-free.app \
  --gitlab-client-id your_gitlab_client_id \
  --gitlab-client-secret your_gitlab_client_secret
```

### Partial Demos

#### GitLab Integration Only
Test GitLab API integration with existing authentication:
```bash
cargo run --example mcp_cli demo gitlab --ngrok-url https://your-ngrok-url.ngrok-free.app
```

#### OAuth Setup Only
Register OAuth provider without full workflow:
```bash
cargo run --example mcp_cli demo setup-oauth \
  --ngrok-url https://your-ngrok-url.ngrok-free.app \
  --gitlab-client-id your_gitlab_client_id \
  --gitlab-client-secret your_gitlab_client_secret
```

## Demo Workflow Breakdown

### 📍 Step 1: Environment Setup and Validation
- Validates NgRok URL and GitLab OAuth credentials
- Prompts for missing configuration
- Sets up the demo environment

### 📍 Step 2: Server Health Check
- Tests connectivity to the MCP server via NgRok tunnel
- Verifies server is responding correctly
- Displays server status and health information

### 📍 Step 3: MCP App Creation
- Creates a new MCP application with demo credentials
- Generates app ID and authentication keys
- Registers the app with the MCP server

### 📍 Step 4: App Installation with Project Context
- Installs the app with appropriate permissions
- Auto-detects GitLab project context from current git repository
- Configures project-specific settings

### 📍 Step 5: OAuth Provider Registration
- Registers GitLab as an OAuth provider
- Configures redirect URIs and scopes
- Sets up OAuth flow parameters

### 📍 Step 6: Authentication Token Creation
- Creates installation tokens for API access
- Stores session information locally
- Configures authentication for subsequent requests

### 📍 Step 7: OAuth Authorization Flow
- Starts local callback server for OAuth
- Opens browser for GitLab authentication
- Handles OAuth callback and token exchange
- Stores access tokens securely

### 📍 Step 8: GitLab API Integration Testing
- Tests user information retrieval
- Validates API connectivity and permissions
- Displays authenticated user details

### 📍 Step 9: Project Context Discovery
- Lists accessible GitLab projects
- Displays project information and metadata
- Shows available operations and permissions

### 📍 Step 10: Issue Management Demo
- Demonstrates issue listing for current project
- Shows issue management capabilities
- Tests project-specific API operations

## Interactive Features

### Breakpoints
Each step includes detailed explanations of what will happen next. You can:
- **Continue**: Proceed to the next step
- **Pause**: Stop the demo and resume later
- **Skip**: Use `--auto-confirm` to skip all prompts

### Project Detection
The demo automatically detects GitLab projects in the current directory by:
- Reading git remote URLs
- Parsing GitLab project paths
- Configuring project context automatically

### Error Handling
Comprehensive error handling with helpful messages:
- Network connectivity issues
- Authentication failures
- API permission problems
- Configuration errors

## Configuration

### Environment Variables
```bash
# Required
export NGROK_URL="https://your-ngrok-url.ngrok-free.app"
export GITLAB_CLIENT_ID="your_gitlab_client_id"
export GITLAB_CLIENT_SECRET="your_gitlab_client_secret"

# Optional
export MCP_SERVER_URL="http://localhost:3000"  # Default server URL
```

### Configuration File
The CLI stores configuration in `~/.mcp-cli.json`:
```json
{
  "server_url": "https://your-ngrok-url.ngrok-free.app",
  "current_session": "demo_session_abc123",
  "sessions": {
    "demo_session_abc123": {
      "session_id": "demo_session_abc123",
      "jwt_token": "eyJ...",
      "installation_id": "demo_inst_xyz789",
      "app_id": "demo_app_def456",
      "expires_at": "2024-01-01T12:00:00Z",
      "created_at": "2024-01-01T11:00:00Z"
    }
  },
  "oauth_tokens": {
    "GitLab": {
      "provider_type": "GitLab",
      "access_token": "glpat-...",
      "refresh_token": null,
      "expires_at": null,
      "scope": [],
      "created_at": "2024-01-01T11:00:00Z"
    }
  }
}
```

## Other CLI Commands

### Server Management
```bash
# Check server status
cargo run --example mcp_cli server status

# List server instances
cargo run --example mcp_cli server list

# Get server information
cargo run --example mcp_cli server info --instance-id gitlab-demo
```

### Authentication Management
```bash
# Show authentication status
cargo run --example mcp_cli auth status

# Create new app
cargo run --example mcp_cli auth create-app --name "My App" --description "Test app"

# Install app
cargo run --example mcp_cli auth install --app-id app_123 --context gitlab:owner/repo

# Login with app
cargo run --example mcp_cli auth login --app-id app_123 --installation-id inst_456

# Logout
cargo run --example mcp_cli auth logout
```

### OAuth Management
```bash
# Register OAuth provider
cargo run --example mcp_cli oauth register \
  --provider gitlab \
  --client-id your_client_id \
  --client-secret your_client_secret \
  --redirect-uri https://your-ngrok-url.ngrok-free.app/mcp/oauth/callback

# Start OAuth authorization
cargo run --example mcp_cli oauth authorize --provider gitlab --installation-id inst_123

# List OAuth tokens
cargo run --example mcp_cli oauth list

# Revoke OAuth token
cargo run --example mcp_cli oauth revoke --provider gitlab
```

### Session Management
```bash
# List sessions
cargo run --example mcp_cli session list

# Show current session
cargo run --example mcp_cli session current

# Switch session
cargo run --example mcp_cli session switch session_123

# Clear all sessions
cargo run --example mcp_cli session clear
```

### Interactive Mode
```bash
# Start interactive mode with menu-driven interface
cargo run --example mcp_cli interactive
```

## Troubleshooting

### Common Issues

#### Server Not Accessible
```
❌ Server health check failed: connection refused
```
**Solution**: Ensure the MCP server is running and NgRok tunnel is active.

#### OAuth Authentication Failed
```
❌ OAuth authorization failed: invalid_client
```
**Solution**: Verify GitLab OAuth client ID and secret are correct.

#### Project Not Detected
```
No GitLab project detected in current directory
```
**Solution**: Run the demo from a directory with a GitLab git remote, or manually specify project context.

#### Token Expired
```
❌ Failed to create auth token: token expired
```
**Solution**: Re-run the authentication flow or clear sessions and start over.

### Debug Mode
Run with verbose output:
```bash
cargo run --example mcp_cli --verbose demo full
```

### Reset Configuration
Clear all stored configuration:
```bash
rm ~/.mcp-cli.json
cargo run --example mcp_cli session clear
```

## Next Steps

After completing the demo:

1. **IDE Integration**: Use the configured MCP server in your IDE (Cursor, Windsurf, etc.)
2. **Additional Projects**: Configure additional GitLab projects
3. **Custom Tools**: Explore other MCP tools and capabilities
4. **Production Setup**: Deploy the MCP server for production use

## Architecture

The demo showcases a complete MCP architecture:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   IDE Client    │    │   MCP Server    │    │  GitLab API     │
│  (Cursor, etc.) │◄──►│  (Rust/Axum)    │◄──►│  (OAuth)        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         ▲                       ▲                       ▲
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   NgRok Tunnel  │    │   Local Config  │    │  Project Context│
│  (Public Access)│    │  (~/.mcp-cli)   │    │  (Git Remote)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

This demonstrates a production-ready MCP setup with:
- **Multi-tenant authentication** via OAuth
- **Secure token management** with JWT
- **Project context awareness** via git integration
- **Public accessibility** via NgRok tunneling
- **IDE integration** via MCP protocol

## Contributing

To extend the demo:

1. Add new demo steps in the `run_full_demo` method
2. Implement additional GitLab API integrations
3. Add support for other OAuth providers (GitHub, etc.)
4. Enhance project context detection
5. Add more comprehensive error handling

The demo is designed to be educational and extensible, providing a solid foundation for understanding and building MCP integrations. 