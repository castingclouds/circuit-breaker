#!/bin/bash

# Circuit Breaker Ollama Setup Script
# This script sets up Ollama with the recommended models for Circuit Breaker integration

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
OLLAMA_PORT=${OLLAMA_PORT:-11434}
OLLAMA_HOST=${OLLAMA_HOST:-127.0.0.1}

# Model configurations based on Circuit Breaker provider definitions
# Using functions instead of associative arrays for better shell compatibility
get_model() {
    case "$1" in
        "lightweight") echo "gemma2:2b" ;;
        "chat") echo "llama3.1:8b" ;;
        "code") echo "qwen2.5-coder:3b" ;;
        "embeddings") echo "nomic-embed-text:latest" ;;
        "fast") echo "gemma2:2b" ;;
        "quality") echo "llama3.1:8b" ;;
        "reasoning") echo "llama3.1:8b" ;;
        "development") echo "qwen2.5-coder:3b" ;;
        *) echo "" ;;
    esac
}

get_model_size() {
    case "$1" in
        "gemma2:2b") echo "1.6GB" ;;
        "llama3.1:8b") echo "4.7GB" ;;
        "qwen2.5-coder:3b") echo "2.0GB" ;;
        "nomic-embed-text:latest") echo "274MB" ;;
        "codellama:7b") echo "3.8GB" ;;
        "mistral:7b") echo "4.1GB" ;;
        "phi3:3.8b") echo "2.3GB" ;;
        *) echo "Unknown" ;;
    esac
}

get_model_description() {
    case "$1" in
        "gemma2:2b") echo "Fast, lightweight model for general tasks" ;;
        "llama3.1:8b") echo "High-quality chat and reasoning model" ;;
        "qwen2.5-coder:3b") echo "Specialized coding assistant model" ;;
        "nomic-embed-text:latest") echo "Text embeddings for semantic search" ;;
        "codellama:7b") echo "Meta's code generation model" ;;
        "mistral:7b") echo "Balanced performance and quality" ;;
        "phi3:3.8b") echo "Microsoft's efficient small model" ;;
        *) echo "Unknown model" ;;
    esac
}

print_header() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                   Circuit Breaker Ollama Setup                ║"
    echo "║              Easy Local AI Model Management Setup             ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
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

check_system_requirements() {
    print_step "Checking system requirements..."
    
    # Check OS
    case "$OSTYPE" in
        linux-gnu*)
            print_success "Linux detected"
            OS_TYPE="linux"
            ;;
        darwin*)
            print_success "macOS detected"
            OS_TYPE="macos"
            # Check macOS version
            macos_version=$(sw_vers -productVersion | cut -d. -f1-2)
            print_step "macOS version: $macos_version"
            ;;
        msys*|cygwin*)
            print_success "Windows detected"
            OS_TYPE="windows"
            ;;
        *)
            print_warning "Unknown OS: $OSTYPE"
            OS_TYPE="unknown"
            ;;
    esac
    
    # Check available disk space (need ~20GB for all models)
    available_space=$(df . | awk 'NR==2 {print $4}')
    if [ $available_space -lt 20971520 ]; then  # 20GB in KB
        print_warning "Less than 20GB available disk space. Large models may not fit."
    fi
    
    # Check memory
    if command -v free &> /dev/null; then
        total_mem=$(free -g | awk 'NR==2{print $2}')
        if [ $total_mem -lt 8 ]; then
            print_warning "Less than 8GB RAM detected. Performance may be limited."
        else
            print_success "Sufficient RAM detected: ${total_mem}GB"
        fi
    elif [[ "$OS_TYPE" == "macos" ]]; then
        # macOS memory check
        total_mem_bytes=$(sysctl -n hw.memsize)
        total_mem_gb=$((total_mem_bytes / 1024 / 1024 / 1024))
        if [ $total_mem_gb -lt 8 ]; then
            print_warning "Less than 8GB RAM detected: ${total_mem_gb}GB. Performance may be limited."
        else
            print_success "Sufficient RAM detected: ${total_mem_gb}GB"
        fi
    fi
}

install_ollama() {
    print_step "Installing Ollama..."
    
    if command -v ollama &> /dev/null; then
        print_success "Ollama already installed"
        ollama --version
        return 0
    fi
    
    case "$OSTYPE" in
        linux-gnu*)
            print_step "Installing Ollama on Linux..."
            curl -fsSL https://ollama.ai/install.sh | sh
            ;;
        darwin*)
            print_step "Installing Ollama on macOS..."
            if command -v brew &> /dev/null; then
                print_step "Installing via Homebrew..."
                brew install ollama
            else
                print_warning "Homebrew not found. Installing via official installer..."
                print_step "Downloading Ollama installer..."
                curl -fsSL https://ollama.ai/install.sh | sh
                
                if [ $? -ne 0 ]; then
                    print_error "Automatic installation failed."
                    echo "Please install manually:"
                    echo "1. Download from https://ollama.ai"
                    echo "2. Or install Homebrew first: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                    exit 1
                fi
            fi
            ;;
        msys*|cygwin*)
            print_error "Please install Ollama manually on Windows from https://ollama.ai"
            echo "Download the Windows installer and run it."
            exit 1
            ;;
        *)
            print_error "Unsupported OS for automatic installation: $OSTYPE"
            echo "Please install Ollama manually from https://ollama.ai"
            exit 1
            ;;
    esac
    
    # Verify installation
    if command -v ollama &> /dev/null; then
        print_success "Ollama installed successfully"
        ollama --version
    else
        print_error "Ollama installation failed"
        exit 1
    fi
}

start_ollama_service() {
    print_step "Starting Ollama service..."
    
    # Check if Ollama is already running
    if curl -s http://localhost:$OLLAMA_PORT > /dev/null 2>&1; then
        print_success "Ollama service is already running"
        return 0
    fi
    
    case "$OSTYPE" in
        linux-gnu*)
            # Start Ollama service on Linux
            if systemctl is-active --quiet ollama 2>/dev/null; then
                print_success "Ollama service is already active"
            else
                print_step "Starting Ollama service..."
                if command -v systemctl &> /dev/null; then
                    sudo systemctl start ollama 2>/dev/null || {
                        print_warning "Could not start systemd service, starting manually..."
                        nohup ollama serve > /dev/null 2>&1 &
                        sleep 3
                    }
                else
                    nohup ollama serve > /dev/null 2>&1 &
                    sleep 3
                fi
            fi
            ;;
        darwin*)
            # Start Ollama on macOS
            if ! pgrep -f "ollama serve" > /dev/null; then
                print_step "Starting Ollama server..."
                nohup ollama serve > /dev/null 2>&1 &
                sleep 3
            fi
            ;;
        *)
            print_step "Starting Ollama server..."
            nohup ollama serve > /dev/null 2>&1 &
            sleep 3
            ;;
    esac
    
    # Wait for service to be ready
    for i in {1..10}; do
        if curl -s http://localhost:$OLLAMA_PORT > /dev/null 2>&1; then
            print_success "Ollama service is running on port $OLLAMA_PORT"
            return 0
        fi
        print_step "Waiting for Ollama service to start... ($i/10)"
        sleep 2
    done
    
    print_error "Failed to start Ollama service"
    exit 1
}

show_model_menu() {
    echo -e "${BLUE}Available models to install:${NC}"
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${YELLOW}macOS optimized selection:${NC}"
    fi
    echo "1) lightweight - $(get_model lightweight) ($(get_model_size "$(get_model lightweight)")) - Fast development/testing ⭐"
    echo "2) chat - $(get_model chat) ($(get_model_size "$(get_model chat)")) - High-quality conversation"
    echo "3) code - $(get_model code) ($(get_model_size "$(get_model code)")) - Code generation and assistance ⭐"
    echo "4) embeddings - $(get_model embeddings) ($(get_model_size "$(get_model embeddings)")) - Text embeddings ⭐"
    echo "5) recommended - Install recommended set ($(get_model lightweight), $(get_model code), $(get_model embeddings))"
    echo "6) all - Install all available models (~12GB total)"
    echo "7) custom - Enter custom model name"
    echo
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${YELLOW}⭐ = Recommended for macOS${NC}"
    fi
}

pull_model() {
    local model_name=$1
    local model_desc=$2
    
    print_step "Pulling model: $model_name"
    echo "Description: $model_desc"
    echo "Size: $(get_model_size "$model_name")"
    
    ollama pull "$model_name"
    
    if [ $? -eq 0 ]; then
        print_success "Model $model_name pulled successfully"
        
        # Test the model with a simple prompt
        print_step "Testing model $model_name..."
        if [[ "$model_name" == *"embed"* ]]; then
            # Test embedding model
            echo "Testing embedding generation..."
            ollama embeddings "$model_name" "Hello world" > /dev/null 2>&1
        else
            # Test chat model
            echo "Testing text generation..."
            ollama run "$model_name" "Hello! Please respond with just 'Hi there!'" --verbose=false 2>/dev/null | head -1
        fi
        
        if [ $? -eq 0 ]; then
            print_success "Model $model_name is working correctly"
        else
            print_warning "Model $model_name pulled but test failed"
        fi
    else
        print_error "Failed to pull model $model_name"
        return 1
    fi
}

install_models() {
    show_model_menu
    
    read -p "Select models to install (1-7): " choice
    
    case $choice in
        1)
            lightweight_model=$(get_model lightweight)
            pull_model "$lightweight_model" "$(get_model_description "$lightweight_model")"
            ;;
        2)
            chat_model=$(get_model chat)
            pull_model "$chat_model" "$(get_model_description "$chat_model")"
            ;;
        3)
            code_model=$(get_model code)
            pull_model "$code_model" "$(get_model_description "$code_model")"
            ;;
        4)
            embeddings_model=$(get_model embeddings)
            pull_model "$embeddings_model" "$(get_model_description "$embeddings_model")"
            ;;
        5)
            print_step "Installing recommended model set..."
            lightweight_model=$(get_model lightweight)
            code_model=$(get_model code)
            embeddings_model=$(get_model embeddings)
            pull_model "$lightweight_model" "$(get_model_description "$lightweight_model")"
            pull_model "$code_model" "$(get_model_description "$code_model")"
            pull_model "$embeddings_model" "$(get_model_description "$embeddings_model")"
            ;;
        6)
            print_step "Installing all models..."
            lightweight_model=$(get_model lightweight)
            chat_model=$(get_model chat)
            code_model=$(get_model code)
            embeddings_model=$(get_model embeddings)
            pull_model "$lightweight_model" "$(get_model_description "$lightweight_model")"
            pull_model "$chat_model" "$(get_model_description "$chat_model")"
            pull_model "$code_model" "$(get_model_description "$code_model")"
            pull_model "$embeddings_model" "$(get_model_description "$embeddings_model")"
            ;;
        7)
            read -p "Enter custom model name (e.g., llama3.2:3b): " custom_model
            pull_model "$custom_model" "Custom model"
            ;;
        *)
            print_error "Invalid choice. Skipping model installation."
            ;;
    esac
}

create_config_files() {
    print_step "Creating configuration files..."
    
    # Create Ollama configuration directory
    mkdir -p ~/.config/circuit-breaker
    
    # Create Ollama environment file
    cat > ~/.config/circuit-breaker/ollama.env << EOF
# Circuit Breaker Ollama Configuration
OLLAMA_BASE_URL=http://localhost:$OLLAMA_PORT
OLLAMA_DEFAULT_MODEL=$(get_model lightweight)
OLLAMA_API_KEY=
OLLAMA_KEEP_ALIVE=5m
OLLAMA_VERIFY_SSL=true
OLLAMA_TIMEOUT_SECONDS=60
EOF
    
    # Create model management script
    cat > ~/.config/circuit-breaker/manage_ollama.sh << 'EOF'
#!/bin/bash

# Circuit Breaker Ollama Management Script

# Source configuration
source ~/.config/circuit-breaker/ollama.env

show_help() {
    echo "Ollama Management Script for Circuit Breaker"
    echo
    echo "Usage: $0 [command] [options]"
    echo
    echo "Commands:"
    echo "  list                List installed models"
    echo "  run <model>         Start interactive chat with model"
    echo "  test <model>        Test model with simple prompt"
    echo "  pull <model>        Download a new model"
    echo "  remove <model>      Remove a model"
    echo "  status              Show Ollama service status"
    echo "  logs                Show recent Ollama logs"
    echo
    echo "Examples:"
    echo "  $0 list"
    echo "  $0 run qwen2.5-coder:3b"
    echo "  $0 test gemma2:2b"
    echo "  $0 pull llama3.2:3b"
}

case "${1:-help}" in
    list)
        echo "Installed Ollama models:"
        ollama list
        ;;
    run)
        if [ -z "$2" ]; then
            echo "Usage: $0 run <model>"
            exit 1
        fi
        echo "Starting interactive chat with $2..."
        ollama run "$2"
        ;;
    test)
        model=${2:-$OLLAMA_DEFAULT_MODEL}
        echo "Testing model: $model"
        if [[ "$model" == *"embed"* ]]; then
            echo "Testing embedding generation..."
            ollama embeddings "$model" "Hello world"
        else
            echo "Testing text generation..."
            ollama run "$model" "Hello! Please respond with just 'Hi there!'" --verbose=false
        fi
        ;;
    pull)
        if [ -z "$2" ]; then
            echo "Usage: $0 pull <model>"
            exit 1
        fi
        echo "Pulling model: $2"
        ollama pull "$2"
        ;;
    remove)
        if [ -z "$2" ]; then
            echo "Usage: $0 remove <model>"
            exit 1
        fi
        echo "Removing model: $2"
        ollama rm "$2"
        ;;
    status)
        echo "Ollama service status:"
        if curl -s http://localhost:11434 > /dev/null 2>&1; then
            echo "✓ Ollama is running on http://localhost:11434"
            echo "API endpoints available:"
            echo "  - http://localhost:11434/api/generate"
            echo "  - http://localhost:11434/api/chat"
            echo "  - http://localhost:11434/api/embeddings"
        else
            echo "✗ Ollama is not running"
            echo "Start with: ollama serve"
        fi
        ;;
    logs)
        echo "Recent Ollama activity:"
        if command -v journalctl &> /dev/null; then
            sudo journalctl -u ollama -n 20 --no-pager
        else
            echo "Logs not available on this system"
        fi
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Unknown command: $1"
        show_help
        exit 1
        ;;
esac
EOF
    
    chmod +x ~/.config/circuit-breaker/manage_ollama.sh
    
    print_success "Configuration files created in ~/.config/circuit-breaker/"
}

test_installation() {
    print_step "Testing Ollama installation..."
    
    # Test Ollama service
    if curl -s http://localhost:$OLLAMA_PORT > /dev/null 2>&1; then
        print_success "✓ Ollama service is accessible"
    else
        print_error "✗ Ollama service is not accessible"
        return 1
    fi
    
    # Test model listing
    print_step "Testing model listing..."
    models=$(ollama list 2>/dev/null | tail -n +2 | wc -l)
    if [ $models -gt 0 ]; then
        print_success "✓ Found $models installed models"
        ollama list
    else
        print_warning "No models installed yet"
    fi
    
    # Test with default model if available
    lightweight_model=$(get_model lightweight)
    if ollama list | grep -q "$lightweight_model"; then
        print_step "Testing chat with $lightweight_model..."
        response=$(echo "Hello! Please respond with just 'Hi there!'" | ollama run "$lightweight_model" --verbose=false 2>/dev/null | head -1)
        if [ $? -eq 0 ]; then
            print_success "✓ Chat test successful"
            echo "Response: $response"
        else
            print_warning "Chat test failed"
        fi
    fi
}

create_examples() {
    print_step "Creating example scripts..."
    
    mkdir -p examples/ollama
    
    # Create basic example
    cat > examples/ollama/basic_chat.py << 'EOF'
#!/usr/bin/env python3
"""
Basic Ollama chat example for Circuit Breaker integration
"""

import requests
import json

def test_ollama_chat():
    url = "http://localhost:11434/api/chat"
    
    payload = {
        "model": "qwen2.5-coder:3b",
        "messages": [
            {"role": "user", "content": "Hello! How are you today?"}
        ],
        "stream": False
    }
    
    try:
        response = requests.post(url, json=payload)
        response.raise_for_status()
        
        result = response.json()
        print("Response:", result["message"]["content"])
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure Ollama is running: ollama serve")
        print("And the model is installed: ollama pull qwen2.5-coder:3b")

def test_circuit_breaker_integration():
    """Test via Circuit Breaker's OpenAI-compatible endpoint"""
    url = "http://localhost:3000/v1/chat/completions"
    
    payload = {
        "model": "ollama://qwen2.5-coder:3b",
        "messages": [
            {"role": "user", "content": "Write a simple Python function to add two numbers"}
        ],
        "max_tokens": 200
    }
    
    try:
        response = requests.post(url, json=payload)
        response.raise_for_status()
        
        result = response.json()
        print("Circuit Breaker Response:", result["choices"][0]["message"]["content"])
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure Circuit Breaker server is running: cargo run --bin server")

if __name__ == "__main__":
    print("Testing direct Ollama connection:")
    test_ollama_chat()
    print("\nTesting Circuit Breaker integration:")
    test_circuit_breaker_integration()
EOF

    # Create streaming example
    cat > examples/ollama/streaming_chat.py << 'EOF'
#!/usr/bin/env python3
"""
Streaming Ollama chat example for Circuit Breaker integration
"""

import requests
import json

def test_ollama_streaming():
    url = "http://localhost:11434/api/chat"
    
    payload = {
        "model": "gemma2:2b",
        "messages": [
            {"role": "user", "content": "Write a short story about a robot learning to paint."}
        ],
        "stream": True
    }
    
    try:
        response = requests.post(url, json=payload, stream=True)
        response.raise_for_status()
        
        print("Streaming response:")
        for line in response.iter_lines():
            if line:
                data = json.loads(line)
                if not data.get("done", False):
                    content = data["message"]["content"]
                    print(content, end="", flush=True)
                else:
                    break
        print("\n")
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure Ollama is running and model is installed")

def test_embeddings():
    url = "http://localhost:11434/api/embeddings"
    
    payload = {
        "model": "nomic-embed-text:latest",
        "prompt": "Hello, world! This is a test for embeddings."
    }
    
    try:
        response = requests.post(url, json=payload)
        response.raise_for_status()
        
        result = response.json()
        embedding = result["embedding"]
        print(f"Embedding generated: {len(embedding)} dimensions")
        print(f"First 5 values: {embedding[:5]}")
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure nomic-embed-text model is installed: ollama pull nomic-embed-text")

if __name__ == "__main__":
    print("Testing streaming chat:")
    test_ollama_streaming()
    print("\nTesting embeddings:")
    test_embeddings()
EOF

    chmod +x examples/ollama/*.py
    
    print_success "Example scripts created in examples/ollama/"
}

show_completion_message() {
    echo -e "${GREEN}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                    Setup Complete! 🎉                         ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    echo -e "${BLUE}Quick Start Commands:${NC}"
    echo
    echo "1. List installed models:"
    echo "   ollama list"
    echo
    echo "2. Chat with a model:"
    echo "   ollama run $(get_model lightweight)"
    echo "   ollama run $(get_model code)"
    echo
    echo "3. Test the models:"
    echo "   ~/.config/circuit-breaker/manage_ollama.sh test $(get_model lightweight)"
    echo
    echo "4. Run example scripts:"
    echo "   python examples/ollama/basic_chat.py"
    echo "   python examples/ollama/streaming_chat.py"
    echo
    echo "5. Start Circuit Breaker server:"
    echo "   cd circuit-breaker && cargo run --bin server"
    echo
    echo "6. Test Circuit Breaker with Ollama:"
    echo "   curl http://localhost:3000/v1/chat/completions \\"
    echo "     -H 'Content-Type: application/json' \\"
    echo "     -d '{\"model\": \"ollama://$(get_model code)\", \"messages\": [{\"role\": \"user\", \"content\": \"Hello!\"}]}'"
    echo
    echo -e "${YELLOW}Management Tools:${NC}"
    echo "• Ollama management: ~/.config/circuit-breaker/manage_ollama.sh"
    echo "• Configuration: ~/.config/circuit-breaker/ollama.env"
    echo "• Examples: examples/ollama/"
    echo
    echo -e "${BLUE}Model Recommendations:${NC}"
    echo "• Development: $(get_model code) (coding tasks)"
    echo "• Chat: $(get_model chat) (general conversation)"
    echo "• Lightweight: $(get_model lightweight) (fast responses)"
    echo "• Embeddings: $(get_model embeddings) (semantic search)"
    echo
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${YELLOW}macOS Performance Tips:${NC}"
        echo "• Ollama works excellently on both Apple Silicon and Intel Macs"
        echo "• Apple Silicon (M1/M2/M3) provides better performance than Intel"
        echo "• All models run efficiently with Ollama's optimized runtime"
        echo "• Use 'ollama run model-name' for interactive chat sessions"
        echo
    fi
    echo -e "${BLUE}📚 Documentation: docs/OLLAMA_INTEGRATION.md${NC}"
}

main() {
    print_header
    
    # Check if help requested
    if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
        echo "Circuit Breaker Ollama Setup Script"
        echo
        echo "Usage: $0 [options]"
        echo
        echo "Options:"
        echo "  --help, -h              Show this help message"
        echo "  --skip-models          Skip model installation"
        echo "  --skip-test            Skip installation test"
        echo "  --port PORT            Set Ollama server port (default: 11434)"
        echo
        echo "Environment Variables:"
        echo "  OLLAMA_PORT            Server port (default: 11434)"
        echo "  OLLAMA_HOST            Server host (default: 127.0.0.1)"
        echo
        exit 0
    fi
    
    # Parse command line arguments
    SKIP_MODELS=false
    SKIP_TEST=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-models)
                SKIP_MODELS=true
                shift
                ;;
            --skip-test)
                SKIP_TEST=true
                shift
                ;;
            --port)
                OLLAMA_PORT="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Run setup steps
    check_system_requirements
    install_ollama
    start_ollama_service
    
    if [ "$SKIP_MODELS" = false ]; then
        install_models
    else
        print_warning "Skipping model installation (--skip-models flag)"
    fi
    
    create_config_files
    create_examples
    
    if [ "$SKIP_TEST" = false ]; then
        test_installation
    else
        print_warning "Skipping installation test (--skip-test flag)"
    fi
    
    show_completion_message
}

# Run main function
main "$@"