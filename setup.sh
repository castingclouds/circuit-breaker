#!/bin/bash

# Circuit Breaker Setup Script
# Automates initial setup for development environment

set -e  # Exit on any error

echo "🚀 Circuit Breaker Setup Script"
echo "==============================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in project root
if [ ! -f "Cargo.toml" ] || [ ! -d "src" ]; then
    print_error "This script must be run from the circuit-breaker project root directory"
    exit 1
fi

print_status "Setting up Circuit Breaker development environment..."

# Step 1: Check Rust installation
print_status "Checking Rust installation..."
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install Rust first:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

RUST_VERSION=$(rustc --version)
print_success "Rust found: $RUST_VERSION"

# Step 2: Check Node.js for TypeScript examples
print_status "Checking Node.js installation..."
if command -v node &> /dev/null; then
    NODE_VERSION=$(node --version)
    print_success "Node.js found: $NODE_VERSION"
    HAS_NODE=true
else
    print_warning "Node.js not found. TypeScript examples will not be available."
    print_warning "Install Node.js 18+ to run TypeScript client examples."
    HAS_NODE=false
fi

# Step 3: Setup environment variables
print_status "Setting up environment configuration..."

if [ ! -f ".env" ]; then
    if [ -f ".env.example" ]; then
        cp .env.example .env
        print_success "Created .env file from .env.example"
        
        echo ""
        print_warning "⚠️  IMPORTANT: Configure your API keys in .env file!"
        echo ""
        echo "Edit .env and add your API key for AI agent functionality:"
        echo "  ANTHROPIC_API_KEY=your_anthropic_api_key_here"
        echo ""
        echo "Optional alternative providers (uncomment in .env if needed):"
        echo "  OPENAI_API_KEY=your_openai_api_key_here"
        echo "  GOOGLE_API_KEY=your_google_api_key_here"
        echo ""
        echo "Get API keys from:"
        echo "  Anthropic (Primary): https://console.anthropic.com/"
        echo "  OpenAI (Alt):        https://platform.openai.com/api-keys"
        echo "  Google (Alt):        https://makersuite.google.com/app/apikey"
        echo ""
    else
        print_error ".env.example file not found!"
        exit 1
    fi
else
    print_success ".env file already exists"
fi

# Step 3.5: Configure storage backend
print_status "Configuring storage backend..."
echo ""
echo "Storage options:"
echo "  1. In-memory (default) - Fast, no persistence"
echo "  2. NATS JetStream - Distributed, persistent"
echo ""
read -p "Select storage backend [1-2] (default: 1): " STORAGE_CHOICE

case "${STORAGE_CHOICE:-1}" in
    2)
        print_status "Configuring NATS storage..."
        
        # Check if NATS is running
        if curl -s http://localhost:8222/varz > /dev/null 2>&1; then
            print_success "NATS server detected on localhost:8222"
        else
            print_warning "NATS server not detected. You'll need to start it manually:"
            echo "  nats-server --jetstream --http_port 8222"
            echo "  Or with Docker: docker run -p 4222:4222 -p 8222:8222 nats:alpine --jetstream --http_port 8222"
        fi
        
        # Set environment variables for NATS
        export STORAGE_BACKEND=nats
        export NATS_URL=nats://localhost:4222
        
        # Add to .env file for persistence
        if ! grep -q "STORAGE_BACKEND" .env; then
            echo "" >> .env
            echo "# Storage Configuration" >> .env
            echo "STORAGE_BACKEND=nats" >> .env
            echo "NATS_URL=nats://localhost:4222" >> .env
        else
            sed -i '' 's/STORAGE_BACKEND=.*/STORAGE_BACKEND=nats/' .env
            if ! grep -q "NATS_URL" .env; then
                echo "NATS_URL=nats://localhost:4222" >> .env
            fi
        fi
        
        print_success "NATS storage configured and saved to .env"
        echo "  STORAGE_BACKEND=nats"
        echo "  NATS_URL=nats://localhost:4222"
        ;;
    1|*)
        print_success "In-memory storage configured (default)"
        export STORAGE_BACKEND=memory
        
        # Add to .env file for persistence
        if ! grep -q "STORAGE_BACKEND" .env; then
            echo "" >> .env
            echo "# Storage Configuration" >> .env
            echo "STORAGE_BACKEND=memory" >> .env
        else
            sed -i '' 's/STORAGE_BACKEND=.*/STORAGE_BACKEND=memory/' .env
        fi
        ;;
esac

# Step 4: Build Rust project
print_status "Building Rust project..."
if cargo build; then
    print_success "Rust build completed successfully"
else
    print_error "Rust build failed"
    exit 1
fi

# Step 5: Setup TypeScript dependencies if Node.js is available
if [ "$HAS_NODE" = true ]; then
    print_status "Setting up TypeScript examples..."
    cd examples/typescript
    if npm install; then
        print_success "TypeScript dependencies installed"
    else
        print_warning "Failed to install TypeScript dependencies"
    fi
    cd ../..
fi

# Step 6: Run tests
print_status "Running tests to verify setup..."
if cargo test; then
    print_success "All tests passed!"
else
    print_warning "Some tests failed, but setup can continue"
fi

# Make setup script executable
chmod +x setup.sh

echo ""
echo "🎉 Setup completed successfully!"
echo ""
echo "Next steps:"
echo "1. Configure ANTHROPIC_API_KEY in .env file (if using AI agents)"
if [ "${STORAGE_BACKEND}" = "nats" ]; then
    echo "2. Start NATS server: nats-server --jetstream --http_port 8222"
    echo "3. Start Circuit Breaker: cargo run --bin server"
    echo "4. Run NATS demo: cargo run --example nats_demo"
else
    echo "2. Start the server: cargo run --bin server"
    echo "3. In another terminal, run demos: cargo run --example places_ai_agent_demo"
fi
echo ""
echo "Storage Backend: ${STORAGE_BACKEND:-memory}"
if [ "${STORAGE_BACKEND}" = "nats" ]; then
    echo "NATS URL: ${NATS_URL:-nats://localhost:4222}"
fi
echo ""
echo "Documentation:"
echo "  README.md                       - Project overview"
echo "  docs/AGENT_CONFIGURATION.md    - AI agent configuration"
echo "  docs/FUNCTION_RUNNER.md        - Function execution"
echo "  docs/RULES_ENGINE.md           - Rules and conditions"
echo "  docs/NATS_IMPLEMENTATION.md    - NATS integration details"
echo ""
echo "Storage Options:"
echo "  • Memory: Fast, ephemeral storage (good for development)"
echo "  • NATS: Distributed, persistent storage (good for production)"
echo ""
echo "To switch storage backends later, run ./setup.sh again"
echo ""
echo "GraphQL Playground will be available at: http://localhost:4000"
echo ""
print_success "Happy coding! 🦀"