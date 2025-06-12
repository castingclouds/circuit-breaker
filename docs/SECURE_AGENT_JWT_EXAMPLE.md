# Secure Agent JWT Authentication Example

This example demonstrates the complete JWT authentication flow for MCP agents, showing how to securely authenticate with the Circuit Breaker MCP server using the GitHub Apps-inspired authentication model described in [SECURE_MCP_SERVER.md](SECURE_MCP_SERVER.md).

## Overview

The `secure_agent_jwt.rs` example consolidates and replaces the previous CLI examples (`cli_usage_demo.rs` and `mcp_cli.rs`) with a focused demonstration of JWT-based authentication for AI agents.

## Authentication Flow

The example demonstrates the complete authentication flow:

1. **App Registration**: Create MCP app with RSA key pair generation
2. **App Installation**: Install app to organization/user account  
3. **JWT Generation**: Create short-lived app JWT using private key
4. **Session Token**: Exchange app JWT for session access token
5. **API Requests**: Use session token for authenticated MCP operations
6. **Token Refresh**: Handle token expiration and renewal

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │ Circuit Breaker │    │  External APIs  │
│   (AI Agent)    │    │   MCP Server    │    │ (GitLab, etc.)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │ 1. Generate App JWT   │                       │
         │ (RSA private key)     │                       │
         │──────────────────────▶│                       │
         │                       │                       │
         │ 2. Session Token      │                       │
         │ (1-hour expiry)       │                       │
         │◀──────────────────────│                       │
         │                       │                       │
         │ 3. MCP Operations     │ 4. External API       │
         │ (Bearer token)        │    Calls              │
         │──────────────────────▶│──────────────────────▶│
         │                       │                       │
         │ 5. Results            │ 6. API Responses      │
         │◀──────────────────────│◀──────────────────────│
```

## Usage

### Prerequisites

1. **MCP Server Running**: The Circuit Breaker MCP server must be running
2. **Dependencies**: All required Rust dependencies are included in the main Cargo.toml

### Running the Example

```bash
# Complete JWT authentication flow demo
cargo run --example secure_agent_jwt demo full

# JWT generation and validation only
cargo run --example secure_agent_jwt demo jwt-only

# Session management demo
cargo run --example secure_agent_jwt demo session-mgmt

# Token refresh demo
cargo run --example secure_agent_jwt demo token-refresh

# Interactive mode
cargo run --example secure_agent_jwt interactive
```

### Command Line Options

```bash
# Specify custom server URL
cargo run --example secure_agent_jwt -- --server-url http://localhost:8080 demo full

# Use custom config file
cargo run --example secure_agent_jwt -- --config ~/.my-jwt-config.json demo full

# Enable verbose output
cargo run --example secure_agent_jwt -- --verbose demo full

# Auto-confirm mode (skip prompts)
cargo run --example secure_agent_jwt demo full --auto-confirm
```

### JWT Operations

```bash
# Generate RSA key pair
cargo run --example secure_agent_jwt jwt generate-keys --output-dir ./keys

# Generate app JWT from private key
cargo run --example secure_agent_jwt jwt generate \
  --app-id "my-app-123" \
  --private-key ./keys/private_key.pem

# Validate JWT with public key
cargo run --example secure_agent_jwt jwt validate \
  --token "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9..." \
  --public-key ./keys/public_key.pem
```

### Session Management

```bash
# List active sessions
cargo run --example secure_agent_jwt session list

# Create session from app JWT
cargo run --example secure_agent_jwt session create \
  --app-jwt "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9..." \
  --installation-id "inst_123"
```

## Demo Walkthrough

### 1. Complete Authentication Flow

The full demo (`demo full`) walks through all steps with interactive breakpoints:

```bash
cargo run --example secure_agent_jwt demo full
```

**Step 1: Environment Setup**
- Validates MCP server connectivity
- Checks server health endpoint

**Step 2: MCP App Creation**
- Generates 2048-bit RSA key pair
- Creates app with permissions configuration
- Returns app_id, client_id, and credentials

**Step 3: App Installation**
- Installs app to user/organization
- Creates installation context
- Returns installation_id

**Step 4: JWT Generation**
- Creates short-lived app JWT (10-minute expiry)
- Signs with RSA private key
- Includes proper claims (iss, aud, iat, exp)

**Step 5: Session Token Creation**
- Exchanges app JWT for session token
- Creates 1-hour session token
- Stores session data with permissions

**Step 6: MCP Operations Testing**
- Tests `tools/list` endpoint
- Tests `tools/call` with echo tool
- Tests `resources/list` endpoint
- Demonstrates authenticated API calls

**Step 7: Token Validation**
- Validates JWT signature
- Extracts and displays claims
- Demonstrates token verification

**Step 8: Session Management**
- Lists active sessions
- Shows session status and expiry
- Demonstrates session tracking

### 2. JWT-Only Demo

Focuses purely on JWT generation and validation:

```bash
cargo run --example secure_agent_jwt demo jwt-only
```

- Generates RSA key pair
- Creates and signs JWT
- Validates JWT signature
- Tests invalid JWT rejection

### 3. Session Management Demo

Demonstrates session lifecycle management:

```bash
cargo run --example secure_agent_jwt demo session-mgmt
```

- Creates multiple demo sessions
- Lists all active sessions
- Shows session status and expiry
- Demonstrates session validation

### 4. Token Refresh Demo

Shows token refresh patterns:

```bash
cargo run --example secure_agent_jwt demo token-refresh
```

- Checks current session expiry
- Demonstrates proactive refresh
- Shows expired token handling
- Creates new session tokens

## Configuration

The example uses a JSON configuration file (default: `~/.secure-agent-jwt.json`) to store:

```json
{
  "server_url": "http://localhost:3000",
  "current_app": {
    "app_id": "app_123",
    "name": "JWT Demo App",
    "private_key": "-----BEGIN PRIVATE KEY-----\n...",
    "public_key": "-----BEGIN RSA PUBLIC KEY-----\n...",
    "client_id": "client_123",
    "client_secret": "secret_456",
    "created_at": "2024-01-01T00:00:00Z",
    "installations": [...]
  },
  "current_session": {
    "session_id": "sess_789",
    "app_id": "app_123",
    "installation_id": "inst_456",
    "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "expires_at": "2024-01-01T01:00:00Z",
    "created_at": "2024-01-01T00:00:00Z",
    "permissions": {...}
  },
  "apps": {...},
  "sessions": {...}
}
```

## Security Features

### RSA Key Management
- 2048-bit RSA key generation
- PKCS#8 private key format
- PKCS#1 public key format
- Secure key storage and handling

### JWT Security
- RS256 algorithm (RSA + SHA256)
- Short-lived app JWTs (10 minutes)
- Proper audience validation
- Timestamp validation (iat, exp)

### Session Management
- 1-hour session token expiry
- Automatic token refresh
- Session tracking and validation
- Secure token storage

### Permission Scoping
- Fine-grained permission control
- API endpoint restrictions
- Resource access limits
- Audit trail logging

## Integration with MCP Server

The example demonstrates integration with the Circuit Breaker MCP server endpoints:

### Authentication Endpoints
- `POST /api/v1/mcp/apps` - Create MCP app
- `POST /api/v1/mcp/apps/{app_id}/installations` - Install app
- `POST /api/v1/mcp/apps/{app_id}/installations/{installation_id}/access_tokens` - Create session token

### MCP Protocol Endpoints
- `POST /mcp/v1/transport/http` - MCP operations
- `GET /health` - Server health check

### Supported MCP Methods
- `tools/list` - List available tools
- `tools/call` - Execute tools
- `resources/list` - List resources
- `resources/read` - Read resource content

## Error Handling

The example includes comprehensive error handling:

- **Network Errors**: Connection failures, timeouts
- **Authentication Errors**: Invalid JWTs, expired tokens
- **Authorization Errors**: Insufficient permissions
- **Validation Errors**: Malformed requests, invalid data
- **Server Errors**: MCP server unavailable, internal errors

## Testing

The example includes unit tests for core functionality:

```bash
# Run tests
cargo test --example secure_agent_jwt

# Test specific functionality
cargo test --example secure_agent_jwt test_jwt_generation_and_validation
cargo test --example secure_agent_jwt test_rsa_key_generation
cargo test --example secure_agent_jwt test_jwt_config_serialization
```

## Comparison with Previous Examples

This example replaces and consolidates:

### `cli_usage_demo.rs` (Removed)
- Programmatic CLI usage patterns
- Multiple demo scenarios
- Session management examples

### `mcp_cli.rs` (Removed)  
- Full CLI implementation
- Interactive mode
- OAuth integration
- Complex command structure

### Benefits of Consolidation
- **Focused Scope**: Pure JWT authentication demo
- **Educational Value**: Clear step-by-step walkthrough
- **Maintainability**: Single example to maintain
- **Documentation**: Comprehensive inline documentation
- **Testing**: Included unit tests

## Related Documentation

- [SECURE_MCP_SERVER.md](SECURE_MCP_SERVER.md) - Complete MCP server architecture
- [REMOTE_MCP_OAUTH_EXAMPLE.md](REMOTE_MCP_OAUTH_EXAMPLE.md) - OAuth integration example
- [WEBHOOK_INTEGRATION_PATTERNS.md](WEBHOOK_INTEGRATION_PATTERNS.md) - Webhook patterns

## Troubleshooting

### Common Issues

**Server Connection Failed**
```
❌ MCP Server is not accessible at: http://localhost:3000
```
- Ensure the MCP server is running
- Check the server URL configuration
- Verify network connectivity

**JWT Validation Failed**
```
❌ JWT validation failed: Invalid signature
```
- Verify the public key matches the private key
- Check JWT format and encoding
- Ensure proper algorithm (RS256)

**Session Expired**
```
❌ Session has expired
```
- Run token refresh demo
- Create new session token
- Check system clock synchronization

**Permission Denied**
```
❌ Insufficient permissions for operation
```
- Check app installation permissions
- Verify session token scope
- Review MCP server configuration

### Debug Mode

Enable verbose output for detailed debugging:

```bash
cargo run --example secure_agent_jwt -- --verbose demo full
```

This provides:
- HTTP request/response details
- JWT token contents (truncated)
- Configuration file operations
- Step-by-step execution logs

## Future Enhancements

Potential improvements for the example:

1. **WebSocket Support**: Demonstrate WebSocket MCP transport
2. **Batch Operations**: Multiple API calls in sequence
3. **Error Recovery**: Automatic retry and fallback patterns
4. **Performance Metrics**: Request timing and throughput
5. **Advanced Permissions**: Complex permission scenarios
6. **Multi-Tenant**: Multiple app/installation management

## Contributing

When modifying this example:

1. **Maintain Educational Value**: Keep the step-by-step structure
2. **Update Documentation**: Reflect changes in this README
3. **Add Tests**: Include unit tests for new functionality
4. **Error Handling**: Provide clear error messages
5. **Security**: Follow JWT and cryptographic best practices 