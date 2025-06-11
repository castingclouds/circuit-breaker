# MCP CLI - Multi-Context Protocol Command Line Interface

The MCP CLI provides a comprehensive command-line interface for interacting with the Multi-Context Protocol (MCP) server, including authentication, OAuth provider management, and server operations.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Commands](#commands)
  - [Server Commands](#server-commands)
  - [Authentication Commands](#authentication-commands)
  - [OAuth Commands](#oauth-commands)
  - [Session Commands](#session-commands)
- [Interactive Mode](#interactive-mode)
- [Authentication Flow](#authentication-flow)
- [OAuth Integration](#oauth-integration)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Installation

Build the CLI from source:

```bash
cd circuit-breaker
cargo build --example mcp_cli --release
```

The binary will be available at `target/release/examples/mcp_cli`.

## Quick Start

1. **Start the MCP server:**
   ```bash
   cargo run --bin server
   ```

2. **Check server status:**
   ```bash
   cargo run --example mcp_cli -- server status
   ```

3. **Create and install an app:**
   ```bash
   cargo run --example mcp_cli -- auth create-app --name "my-app" --description "My MCP application"
   cargo run --example mcp_cli -- auth install --app-id <app-id> --context "my-context"
   ```

4. **Login:**
   ```bash
   cargo run --example mcp_cli -- auth login --app-id <app-id> --installation-id <installation-id>
   ```

5. **Try interactive mode:**
   ```bash
   cargo run --example mcp_cli -- interactive
   ```

## Configuration

The CLI stores configuration in `~/.mcp-cli.json` by default. You can specify a different location:

```bash
cargo run --example mcp_cli -- --config /path/to/config.json <command>
```

### Configuration File Structure

```json
{
  "server_url": "http://localhost:3000",
  "current_session": "session-123",
  "sessions": {
    "session-123": {
      "session_id": "session-123",
      "jwt_token": "eyJ...",
      "installation_id": "install-456",
      "app_id": "app-789",
      "expires_at": "2024-01-01T12:00:00Z",
      "created_at": "2024-01-01T10:00:00Z"
    }
  },
  "oauth_tokens": {
    "gitlab": {
      "provider_type": "gitlab",
      "access_token": "glpat-...",
      "refresh_token": "refresh-...",
      "expires_at": "2024-01-01T14:00:00Z",
      "scope": ["read_user", "read_repository"],
      "created_at": "2024-01-01T12:00:00Z"
    }
  }
}
```

## Commands

### Global Options

- `--server-url <URL>`: MCP server URL (default: http://localhost:3000)
- `--config <PATH>`: Configuration file path (default: ~/.mcp-cli.json)
- `--verbose`: Enable verbose output
- `--help`: Show help information

### Server Commands

#### `mcp-cli server status`
Check the server status and health.

```bash
cargo run --example mcp_cli -- server status
```

#### `mcp-cli server list`
List all server instances.

```bash
cargo run --example mcp_cli -- server list
```

#### `mcp-cli server info`
Get detailed information about a server instance.

```bash
cargo run --example mcp_cli -- server info --instance-id <id>
```

### Authentication Commands

#### `mcp-cli auth create-app`
Create a new MCP application.

```bash
cargo run --example mcp_cli -- auth create-app --name "my-app" --description "My application"
```

#### `mcp-cli auth install`
Install an application in a specific context.

```bash
cargo run --example mcp_cli -- auth install --app-id <app-id> --context "production"
```

#### `mcp-cli auth login`
Authenticate and create a session.

```bash
cargo run --example mcp_cli -- auth login --app-id <app-id> --installation-id <installation-id>
```

#### `mcp-cli auth logout`
Logout and revoke the current session.

```bash
cargo run --example mcp_cli -- auth logout
```

#### `mcp-cli auth status`
Show current authentication status.

```bash
cargo run --example mcp_cli -- auth status
```

#### `mcp-cli auth list-apps`
List all registered applications.

```bash
cargo run --example mcp_cli -- auth list-apps
```

### OAuth Commands

#### `mcp-cli oauth register`
Register an OAuth provider.

```bash
cargo run --example mcp_cli -- oauth register \
  --provider gitlab \
  --client-id <client-id> \
  --client-secret <client-secret> \
  --redirect-uri http://localhost:3000/callback \
  --scopes "read_user,read_repository"
```

Supported providers:
- `gitlab`: GitLab OAuth
- `github`: GitHub OAuth
- `google`: Google OAuth

#### `mcp-cli oauth authorize`
Start OAuth authorization flow.

```bash
cargo run --example mcp_cli -- oauth authorize --provider gitlab --installation-id <installation-id>
```

This command will:
1. Generate an authorization URL
2. Open your default browser
3. Display the state parameter for verification

#### `mcp-cli oauth callback`
Complete OAuth authorization with callback data.

```bash
cargo run --example mcp_cli -- oauth callback --code <authorization-code> --state <state-parameter>
```

#### `mcp-cli oauth list`
List all OAuth tokens.

```bash
cargo run --example mcp_cli -- oauth list
```

#### `mcp-cli oauth revoke`
Revoke an OAuth token.

```bash
cargo run --example mcp_cli -- oauth revoke --provider gitlab
```

### Session Commands

#### `mcp-cli session list`
List all active sessions.

```bash
cargo run --example mcp_cli -- session list
```

#### `mcp-cli session current`
Show current session details.

```bash
cargo run --example mcp_cli -- session current
```

#### `mcp-cli session switch`
Switch to a different session.

```bash
cargo run --example mcp_cli -- session switch <session-id>
```

#### `mcp-cli session clear`
Clear all sessions (with confirmation).

```bash
cargo run --example mcp_cli -- session clear
```

## Interactive Mode

The CLI provides an interactive mode for easier navigation:

```bash
cargo run --example mcp_cli -- interactive
```

Interactive mode features:
- Menu-driven navigation
- Input validation
- Progress indicators
- Contextual help

### Interactive Mode Structure

```
Main Menu
├── Server Status
├── Authentication
│   ├── Show Auth Status
│   ├── Create App
│   ├── Install App
│   ├── Login
│   ├── Logout
│   └── List Apps
├── OAuth Management
│   ├── List OAuth Tokens
│   ├── Register Provider
│   ├── Start Authorization
│   ├── Complete Callback
│   └── Revoke Token
├── Session Management
│   ├── List Sessions
│   ├── Show Current Session
│   ├── Switch Session
│   └── Clear All Sessions
└── Exit
```

## Authentication Flow

The MCP authentication system follows a multi-step process:

### 1. App Registration
```bash
# Create a new app
cargo run --example mcp_cli -- auth create-app --name "my-app" --description "My application"

# Response includes app_id for next steps
```

### 2. App Installation
```bash
# Install the app in a context
cargo run --example mcp_cli -- auth install --app-id <app-id> --context "production"

# Response includes installation_id for authentication
```

### 3. Authentication
```bash
# Login to create a session
cargo run --example mcp_cli -- auth login --app-id <app-id> --installation-id <installation-id>

# Receives JWT token for API access
```

### 4. API Access
All subsequent API calls use the JWT token automatically:
```bash
# These commands use the stored JWT token
cargo run --example mcp_cli -- server list
cargo run --example mcp_cli -- oauth list
```

## OAuth Integration

The CLI supports OAuth integration with multiple providers for accessing external APIs.

### Supported Providers

#### GitLab
```bash
cargo run --example mcp_cli -- oauth register \
  --provider gitlab \
  --client-id <gitlab-app-id> \
  --client-secret <gitlab-app-secret> \
  --redirect-uri http://localhost:3000/callback \
  --scopes "read_user,read_repository,read_api"
```

#### GitHub
```bash
cargo run --example mcp_cli -- oauth register \
  --provider github \
  --client-id <github-app-id> \
  --client-secret <github-app-secret> \
  --redirect-uri http://localhost:3000/callback \
  --scopes "user:read,repo:read"
```

#### Google
```bash
cargo run --example mcp_cli -- oauth register \
  --provider google \
  --client-id <google-client-id> \
  --client-secret <google-client-secret> \
  --redirect-uri http://localhost:3000/callback \
  --scopes "openid,profile,email"
```

### OAuth Flow

1. **Register Provider**: Configure OAuth app credentials
2. **Start Authorization**: Generate authorization URL and open browser
3. **User Authorization**: User grants permissions in browser
4. **Complete Callback**: Exchange authorization code for access token
5. **API Access**: Use stored tokens for external API calls

## Examples

### Complete Setup Example

```bash
#!/bin/bash

# 1. Check server
cargo run --example mcp_cli -- server status

# 2. Create app
APP_RESPONSE=$(cargo run --example mcp_cli -- auth create-app --name "demo-app" --description "Demo application")
APP_ID=$(echo "$APP_RESPONSE" | jq -r '.app_id')

# 3. Install app
INSTALL_RESPONSE=$(cargo run --example mcp_cli -- auth install --app-id "$APP_ID" --context "demo")
INSTALLATION_ID=$(echo "$INSTALL_RESPONSE" | jq -r '.installation_id')

# 4. Login
cargo run --example mcp_cli -- auth login --app-id "$APP_ID" --installation-id "$INSTALLATION_ID"

# 5. Check status
cargo run --example mcp_cli -- auth status

# 6. Register OAuth provider
cargo run --example mcp_cli -- oauth register \
  --provider gitlab \
  --client-id "$GITLAB_CLIENT_ID" \
  --client-secret "$GITLAB_CLIENT_SECRET" \
  --redirect-uri "http://localhost:3000/callback" \
  --scopes "read_user,read_repository"

# 7. Start OAuth flow
cargo run --example mcp_cli -- oauth authorize --provider gitlab --installation-id "$INSTALLATION_ID"
```

### Multi-Session Management

```bash
# Create multiple sessions
cargo run --example mcp_cli -- auth login --app-id app1 --installation-id install1
cargo run --example mcp_cli -- auth login --app-id app2 --installation-id install2

# List sessions
cargo run --example mcp_cli -- session list

# Switch between sessions
cargo run --example mcp_cli -- session switch session-123
cargo run --example mcp_cli -- session switch session-456

# Check current session
cargo run --example mcp_cli -- session current
```

### OAuth Provider Management

```bash
# Register multiple providers
cargo run --example mcp_cli -- oauth register --provider gitlab --client-id ... --client-secret ...
cargo run --example mcp_cli -- oauth register --provider github --client-id ... --client-secret ...

# Authorize with different providers
cargo run --example mcp_cli -- oauth authorize --provider gitlab --installation-id install1
cargo run --example mcp_cli -- oauth authorize --provider github --installation-id install2

# List all tokens
cargo run --example mcp_cli -- oauth list

# Revoke specific tokens
cargo run --example mcp_cli -- oauth revoke --provider gitlab
```

## Troubleshooting

### Common Issues

#### Server Connection Issues
```bash
# Check if server is running
cargo run --example mcp_cli -- server status

# Verify server URL
cargo run --example mcp_cli -- --server-url http://localhost:3000 server status
```

#### Authentication Issues
```bash
# Check current auth status
cargo run --example mcp_cli -- auth status

# Clear expired sessions
cargo run --example mcp_cli -- session clear

# Login again
cargo run --example mcp_cli -- auth login --app-id <app-id> --installation-id <installation-id>
```

#### Configuration Issues
```bash
# Check config file location
ls -la ~/.mcp-cli.json

# Use different config file
mcp-cli --config /tmp/mcp-cli.json auth status

# Reset configuration
rm ~/.mcp-cli.json
```

#### OAuth Issues
```bash
# List current OAuth tokens
cargo run --example mcp_cli -- oauth list

# Check token expiration
cargo run --example mcp_cli -- oauth list | jq '.tokens[] | select(.expires_at < now)'

# Re-authorize expired tokens
cargo run --example mcp_cli -- oauth authorize --provider gitlab --installation-id <installation-id>
```

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
# Enable verbose output
cargo run --example mcp_cli -- --verbose server status

# Set log level
RUST_LOG=debug cargo run --example mcp_cli -- server status
```

### Error Codes

- `1`: General CLI error
- `2`: Configuration error
- `3`: Authentication error
- `4`: Network/server error
- `5`: OAuth error

### Getting Help

```bash
# General help
cargo run --example mcp_cli -- --help

# Command-specific help
cargo run --example mcp_cli -- auth --help
cargo run --example mcp_cli -- oauth register --help

# Interactive mode provides contextual help
cargo run --example mcp_cli -- interactive
```

## Environment Variables

The CLI supports several environment variables:

- `MCP_SERVER_URL`: Default server URL
- `MCP_CLI_CONFIG`: Default configuration file path
- `RUST_LOG`: Logging level (debug, info, warn, error)

Example:
```bash
export MCP_SERVER_URL="https://mcp.example.com"
export MCP_CLI_CONFIG="$HOME/.config/mcp/cli.json"
export RUST_LOG="info"

cargo run --example mcp_cli -- server status
```

## Security Considerations

1. **Configuration File**: Store in secure location with appropriate permissions
2. **JWT Tokens**: Automatically expire and require re-authentication
3. **OAuth Tokens**: Stored securely with refresh capability
4. **Client Secrets**: Never log or display in plain text
5. **HTTPS**: Use HTTPS for production servers

## Contributing

To contribute to the MCP CLI:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

### Running Tests

```bash
cargo test --example mcp_cli
```

### Code Style

Follow Rust standard formatting:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
```
