#!/bin/bash

# MCP CLI Utility Functions
# This script provides common utility functions for MCP CLI operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default configuration
DEFAULT_SERVER_URL="http://localhost:3000"
DEFAULT_CONFIG="$HOME/.mcp-cli.json"

# Helper function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Helper function to run CLI commands
run_mcp_cli() {
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"
    local server_url="${MCP_SERVER_URL:-$DEFAULT_SERVER_URL}"

    cargo run --example mcp_cli -- --config "$config_file" --server-url "$server_url" "$@"
}

# Check if MCP server is running
check_server() {
    print_color $BLUE "Checking MCP server status..."
    if run_mcp_cli server status > /dev/null 2>&1; then
        print_color $GREEN "✓ MCP server is running"
        return 0
    else
        print_color $RED "✗ MCP server is not running"
        print_color $YELLOW "Start the server with: cargo run --bin server"
        return 1
    fi
}

# Quick setup function
quick_setup() {
    local app_name="${1:-quick-setup-app}"
    local context="${2:-default}"

    print_color $BLUE "=== Quick MCP Setup ==="

    # Check server
    if ! check_server; then
        return 1
    fi

    # Create app
    print_color $BLUE "Creating app: $app_name"
    local app_response=$(run_mcp_cli auth create-app --name "$app_name" --description "Quick setup application")

    if [ $? -eq 0 ]; then
        print_color $GREEN "✓ App created successfully"
        echo "$app_response"
    else
        print_color $RED "✗ Failed to create app"
        return 1
    fi

    print_color $YELLOW "Please save the app_id from the response above for the next steps"
}

# Show current status
show_status() {
    print_color $BLUE "=== MCP CLI Status ==="

    # Server status
    print_color $CYAN "Server Status:"
    run_mcp_cli server status
    echo

    # Auth status
    print_color $CYAN "Authentication Status:"
    run_mcp_cli auth status
    echo

    # Session status
    print_color $CYAN "Session Status:"
    run_mcp_cli session current
    echo

    # OAuth status
    print_color $CYAN "OAuth Status:"
    run_mcp_cli oauth list
}

# Clean up all data
cleanup_all() {
    print_color $YELLOW "WARNING: This will remove all MCP CLI data!"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        print_color $YELLOW "Cleanup cancelled"
        return 0
    fi

    print_color $BLUE "Cleaning up MCP CLI data..."

    # Logout if logged in
    print_color $CYAN "Logging out..."
    run_mcp_cli auth logout || true

    # Clear sessions
    print_color $CYAN "Clearing sessions..."
    echo "yes" | run_mcp_cli session clear || true

    # Remove config file
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"
    if [ -f "$config_file" ]; then
        rm "$config_file"
        print_color $GREEN "✓ Removed config file: $config_file"
    fi

    print_color $GREEN "✓ Cleanup complete"
}

# Register OAuth provider with prompts
register_oauth_provider() {
    local provider_type=$1

    if [ -z "$provider_type" ]; then
        print_color $BLUE "Available OAuth providers:"
        echo "1) gitlab"
        echo "2) github"
        echo "3) google"
        read -p "Select provider (1-3): " choice

        case $choice in
            1) provider_type="gitlab" ;;
            2) provider_type="github" ;;
            3) provider_type="google" ;;
            *) print_color $RED "Invalid choice"; return 1 ;;
        esac
    fi

    print_color $BLUE "Registering $provider_type OAuth provider"

    read -p "Client ID: " client_id
    read -s -p "Client Secret: " client_secret
    echo
    read -p "Redirect URI (default: http://localhost:3000/callback): " redirect_uri
    redirect_uri=${redirect_uri:-"http://localhost:3000/callback"}

    # Provider-specific default scopes
    case $provider_type in
        gitlab)
            default_scopes="read_user,read_repository,read_api"
            ;;
        github)
            default_scopes="user:read,repo:read"
            ;;
        google)
            default_scopes="openid,profile,email"
            ;;
    esac

    read -p "Scopes (default: $default_scopes): " scopes
    scopes=${scopes:-$default_scopes}

    run_mcp_cli oauth register \
        --provider "$provider_type" \
        --client-id "$client_id" \
        --client-secret "$client_secret" \
        --redirect-uri "$redirect_uri" \
        --scopes "$scopes"

    if [ $? -eq 0 ]; then
        print_color $GREEN "✓ OAuth provider registered successfully"
    else
        print_color $RED "✗ Failed to register OAuth provider"
        return 1
    fi
}

# Complete OAuth flow
complete_oauth_flow() {
    local provider_type=$1
    local installation_id=$2

    if [ -z "$provider_type" ]; then
        read -p "Provider type: " provider_type
    fi

    if [ -z "$installation_id" ]; then
        read -p "Installation ID: " installation_id
    fi

    print_color $BLUE "Starting OAuth flow for $provider_type"

    # Start authorization
    run_mcp_cli oauth authorize --provider "$provider_type" --installation-id "$installation_id"

    print_color $YELLOW "Complete the authorization in your browser, then return here"
    read -p "Authorization Code: " auth_code
    read -p "State Parameter: " state_param

    # Complete callback
    run_mcp_cli oauth callback --code "$auth_code" --state "$state_param"

    if [ $? -eq 0 ]; then
        print_color $GREEN "✓ OAuth flow completed successfully"
    else
        print_color $RED "✗ OAuth flow failed"
        return 1
    fi
}

# Backup configuration
backup_config() {
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"
    local backup_file="${config_file}.backup.$(date +%Y%m%d_%H%M%S)"

    if [ -f "$config_file" ]; then
        cp "$config_file" "$backup_file"
        print_color $GREEN "✓ Configuration backed up to: $backup_file"
    else
        print_color $YELLOW "No configuration file found to backup"
    fi
}

# Restore configuration
restore_config() {
    local backup_file=$1
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"

    if [ -z "$backup_file" ]; then
        print_color $BLUE "Available backup files:"
        ls -la "${config_file}.backup."* 2>/dev/null || {
            print_color $YELLOW "No backup files found"
            return 1
        }
        read -p "Enter backup file path: " backup_file
    fi

    if [ -f "$backup_file" ]; then
        cp "$backup_file" "$config_file"
        print_color $GREEN "✓ Configuration restored from: $backup_file"
    else
        print_color $RED "✗ Backup file not found: $backup_file"
        return 1
    fi
}

# Export configuration
export_config() {
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"
    local export_file=$1

    if [ -z "$export_file" ]; then
        export_file="mcp-cli-export-$(date +%Y%m%d_%H%M%S).json"
    fi

    if [ -f "$config_file" ]; then
        # Remove sensitive data before export
        jq 'del(.sessions[].jwt_token) | del(.oauth_tokens[].access_token) | del(.oauth_tokens[].refresh_token)' "$config_file" > "$export_file"
        print_color $GREEN "✓ Configuration exported to: $export_file"
        print_color $YELLOW "Note: Sensitive tokens have been removed from the export"
    else
        print_color $RED "✗ No configuration file found to export"
        return 1
    fi
}

# Health check
health_check() {
    print_color $BLUE "=== MCP CLI Health Check ==="

    # Check if CLI binary exists
    if command -v cargo >/dev/null 2>&1; then
        print_color $GREEN "✓ Cargo is available"
    else
        print_color $RED "✗ Cargo not found"
        return 1
    fi

    # Check if server is running
    check_server

    # Check configuration file
    local config_file="${MCP_CLI_CONFIG:-$DEFAULT_CONFIG}"
    if [ -f "$config_file" ]; then
        print_color $GREEN "✓ Configuration file exists: $config_file"

        # Validate JSON
        if jq empty "$config_file" >/dev/null 2>&1; then
            print_color $GREEN "✓ Configuration file is valid JSON"
        else
            print_color $RED "✗ Configuration file contains invalid JSON"
        fi
    else
        print_color $YELLOW "⚠ Configuration file not found (will be created on first use)"
    fi

    # Check for expired sessions
    if [ -f "$config_file" ]; then
        local expired_sessions=$(jq -r '.sessions | to_entries[] | select(.value.expires_at < now | strftime("%Y-%m-%dT%H:%M:%SZ")) | .key' "$config_file" 2>/dev/null | wc -l)
        if [ "$expired_sessions" -gt 0 ]; then
            print_color $YELLOW "⚠ Found $expired_sessions expired session(s)"
        else
            print_color $GREEN "✓ No expired sessions found"
        fi
    fi

    print_color $GREEN "Health check complete"
}

# Main function to handle command line arguments
main() {
    case "${1:-}" in
        "check")
            check_server
            ;;
        "setup")
            quick_setup "$2" "$3"
            ;;
        "status")
            show_status
            ;;
        "cleanup")
            cleanup_all
            ;;
        "oauth-register")
            register_oauth_provider "$2"
            ;;
        "oauth-flow")
            complete_oauth_flow "$2" "$3"
            ;;
        "backup")
            backup_config
            ;;
        "restore")
            restore_config "$2"
            ;;
        "export")
            export_config "$2"
            ;;
        "health")
            health_check
            ;;
        "help"|*)
            print_color $BLUE "MCP CLI Utilities"
            echo
            echo "Usage: $0 <command> [arguments]"
            echo
            echo "Commands:"
            echo "  check                    Check if MCP server is running"
            echo "  setup [app_name] [ctx]   Quick setup with app creation"
            echo "  status                   Show comprehensive status"
            echo "  cleanup                  Clean up all CLI data"
            echo "  oauth-register [type]    Register OAuth provider interactively"
            echo "  oauth-flow [type] [id]   Complete OAuth authorization flow"
            echo "  backup                   Backup configuration file"
            echo "  restore [backup_file]    Restore configuration from backup"
            echo "  export [export_file]     Export configuration (without secrets)"
            echo "  health                   Run health check"
            echo "  help                     Show this help message"
            echo
            echo "Environment Variables:"
            echo "  MCP_SERVER_URL          Server URL (default: $DEFAULT_SERVER_URL)"
            echo "  MCP_CLI_CONFIG          Config file path (default: $DEFAULT_CONFIG)"
            echo
            echo "Examples:"
            echo "  $0 check"
            echo "  $0 setup my-app production"
            echo "  $0 oauth-register gitlab"
            echo "  $0 backup"
            echo "  $0 health"
            ;;
    esac
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
