#!/bin/bash

# MCP CLI Demo Setup Script
# This script demonstrates the full authentication flow using the MCP CLI

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:3000"
CLI_CONFIG="$HOME/.mcp-cli-demo.json"
APP_NAME="demo-app"
APP_DESCRIPTION="Demo application for MCP CLI testing"

echo -e "${BLUE}=== MCP CLI Demo Setup ===${NC}"
echo "This script will demonstrate the complete MCP authentication flow"
echo "Server URL: $SERVER_URL"
echo "Config file: $CLI_CONFIG"
echo ""

# Function to run CLI commands with error handling
run_cli() {
    echo -e "${YELLOW}Running:${NC} mcp-cli $*"
    if ! cargo run --example mcp_cli -- --config "$CLI_CONFIG" --server-url "$SERVER_URL" "$@"; then
        echo -e "${RED}Command failed: mcp-cli $*${NC}"
        return 1
    fi
    echo ""
}

# Function to wait for user input
wait_for_input() {
    echo -e "${GREEN}Press Enter to continue...${NC}"
    read
}

# Check if server is running
echo -e "${BLUE}Step 1: Checking server status${NC}"
if ! run_cli server status; then
    echo -e "${RED}Error: MCP server is not running at $SERVER_URL${NC}"
    echo "Please start the server first with: cargo run --bin server"
    exit 1
fi

wait_for_input

# Create a new app
echo -e "${BLUE}Step 2: Creating a new MCP app${NC}"
run_cli auth create-app --name "$APP_NAME" --description "$APP_DESCRIPTION"

# Get the app ID from the response (this would need to be parsed in a real scenario)
echo -e "${YELLOW}Note: In a real scenario, you would save the app_id from the response${NC}"
echo "For this demo, please note the app_id from the above response"
echo ""
wait_for_input

# Prompt for app ID
echo -e "${BLUE}Step 3: Installing the app${NC}"
echo "Please enter the app_id from the previous step:"
read -p "App ID: " APP_ID

if [ -z "$APP_ID" ]; then
    echo -e "${RED}Error: App ID is required${NC}"
    exit 1
fi

run_cli auth install --app-id "$APP_ID" --context "demo-context"

echo "Please note the installation_id from the above response"
echo ""
wait_for_input

# Login with the app
echo -e "${BLUE}Step 4: Logging in${NC}"
echo "Please enter the installation_id from the previous step:"
read -p "Installation ID: " INSTALLATION_ID

if [ -z "$INSTALLATION_ID" ]; then
    echo -e "${RED}Error: Installation ID is required${NC}"
    exit 1
fi

run_cli auth login --app-id "$APP_ID" --installation-id "$INSTALLATION_ID"

wait_for_input

# Show authentication status
echo -e "${BLUE}Step 5: Checking authentication status${NC}"
run_cli auth status

wait_for_input

# OAuth provider registration demo
echo -e "${BLUE}Step 6: OAuth Provider Registration Demo${NC}"
echo "This step demonstrates OAuth provider registration."
echo "You'll need OAuth app credentials from GitLab, GitHub, or Google."
echo ""
echo "Would you like to register an OAuth provider? (y/n)"
read -p "Register OAuth provider: " REGISTER_OAUTH

if [ "$REGISTER_OAUTH" = "y" ]; then
    echo "Select provider type:"
    echo "1) GitLab"
    echo "2) GitHub"
    echo "3) Google"
    read -p "Choice (1-3): " PROVIDER_CHOICE

    case $PROVIDER_CHOICE in
        1) PROVIDER_TYPE="gitlab" ;;
        2) PROVIDER_TYPE="github" ;;
        3) PROVIDER_TYPE="google" ;;
        *) echo "Invalid choice"; exit 1 ;;
    esac

    read -p "Client ID: " CLIENT_ID
    read -p "Client Secret: " CLIENT_SECRET
    read -p "Redirect URI (default: http://localhost:3000/callback): " REDIRECT_URI
    REDIRECT_URI=${REDIRECT_URI:-"http://localhost:3000/callback"}
    read -p "Scopes (comma-separated, optional): " SCOPES

    if [ -n "$SCOPES" ]; then
        run_cli oauth register --provider "$PROVIDER_TYPE" --client-id "$CLIENT_ID" --client-secret "$CLIENT_SECRET" --redirect-uri "$REDIRECT_URI" --scopes "$SCOPES"
    else
        run_cli oauth register --provider "$PROVIDER_TYPE" --client-id "$CLIENT_ID" --client-secret "$CLIENT_SECRET" --redirect-uri "$REDIRECT_URI"
    fi

    wait_for_input

    # Start OAuth flow
    echo -e "${BLUE}Step 7: Starting OAuth Authorization Flow${NC}"
    run_cli oauth authorize --provider "$PROVIDER_TYPE" --installation-id "$INSTALLATION_ID"

    echo "The browser should have opened for OAuth authorization."
    echo "After completing authorization, you'll get a callback with an authorization code."
    echo ""
    echo "Would you like to complete the OAuth callback? (y/n)"
    read -p "Complete OAuth callback: " COMPLETE_OAUTH

    if [ "$COMPLETE_OAUTH" = "y" ]; then
        read -p "Authorization Code: " AUTH_CODE
        read -p "State Parameter: " STATE_PARAM

        run_cli oauth callback --code "$AUTH_CODE" --state "$STATE_PARAM"

        wait_for_input

        # List OAuth tokens
        echo -e "${BLUE}Step 8: Listing OAuth Tokens${NC}"
        run_cli oauth list
    fi
fi

wait_for_input

# Session management demo
echo -e "${BLUE}Session Management Demo${NC}"
run_cli session list
run_cli session current

wait_for_input

# Server information
echo -e "${BLUE}Server Information${NC}"
run_cli server list

wait_for_input

# Interactive mode demo
echo -e "${BLUE}Interactive Mode Demo${NC}"
echo "The CLI also supports an interactive mode for easier navigation."
echo "You can start it with: mcp-cli interactive"
echo ""
echo "Would you like to try interactive mode? (y/n)"
read -p "Start interactive mode: " START_INTERACTIVE

if [ "$START_INTERACTIVE" = "y" ]; then
    run_cli interactive
fi

# Cleanup option
echo -e "${BLUE}Demo Complete!${NC}"
echo ""
echo "The demo has completed successfully. You now have:"
echo "- A registered MCP app"
echo "- An active authentication session"
echo "- Optional OAuth provider configuration"
echo ""
echo "Configuration file: $CLI_CONFIG"
echo ""
echo "Would you like to clean up the demo data? (y/n)"
read -p "Clean up: " CLEANUP

if [ "$CLEANUP" = "y" ]; then
    echo "Cleaning up..."

    # Logout
    run_cli auth logout

    # Clear sessions
    run_cli session clear

    # Remove config file
    if [ -f "$CLI_CONFIG" ]; then
        rm "$CLI_CONFIG"
        echo "Removed config file: $CLI_CONFIG"
    fi

    echo -e "${GREEN}Cleanup complete!${NC}"
else
    echo "Demo data preserved. You can continue using the CLI with:"
    echo "  cargo run --example mcp_cli -- --config \"$CLI_CONFIG\" --server-url \"$SERVER_URL\""
fi

echo ""
echo -e "${GREEN}MCP CLI Demo Complete!${NC}"
echo ""
echo "Available commands:"
echo "  cargo run --example mcp_cli -- server status           # Check server status"
echo "  cargo run --example mcp_cli -- auth status            # Check authentication"
echo "  cargo run --example mcp_cli -- oauth list             # List OAuth tokens"
echo "  cargo run --example mcp_cli -- session list           # List sessions"
echo "  cargo run --example mcp_cli -- interactive            # Interactive mode"
echo ""
echo "For full help: cargo run --example mcp_cli -- --help"
