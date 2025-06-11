#!/bin/bash

# Circuit Breaker vLLM Setup Script
# This script sets up vLLM with the recommended models for Circuit Breaker integration

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VLLM_PORT=${VLLM_PORT:-8000}
VLLM_HOST=${VLLM_HOST:-0.0.0.0}
PYTHON_VERSION=${PYTHON_VERSION:-3.12}
CONDA_ENV_NAME=${CONDA_ENV_NAME:-vllm}
GPU_MEMORY_UTILIZATION=${GPU_MEMORY_UTILIZATION:-0.9}

# Model configurations based on Circuit Breaker provider definitions
# Using functions instead of associative arrays for better shell compatibility
get_model() {
    case "$1" in
        "lightweight") echo "microsoft/DialoGPT-medium" ;;
        "chat") echo "meta-llama/Llama-2-7b-chat-hf" ;;
        "code") echo "codellama/CodeLlama-7b-Instruct-hf" ;;
        "embeddings") echo "sentence-transformers/all-MiniLM-L6-v2" ;;
        "fast") echo "Salesforce/codegen-2B-mono" ;;
        "quality") echo "meta-llama/Llama-2-13b-chat-hf" ;;
        *) echo "" ;;
    esac
}

get_model_vram() {
    case "$1" in
        "microsoft/DialoGPT-medium") echo "2GB" ;;
        "meta-llama/Llama-2-7b-chat-hf") echo "8GB" ;;
        "codellama/CodeLlama-7b-Instruct-hf") echo "8GB" ;;
        "sentence-transformers/all-MiniLM-L6-v2") echo "512MB" ;;
        "Salesforce/codegen-2B-mono") echo "4GB" ;;
        "meta-llama/Llama-2-13b-chat-hf") echo "16GB" ;;
        *) echo "Unknown" ;;
    esac
}

print_header() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                    Circuit Breaker vLLM Setup                 ║"
    echo "║        High-Performance Local AI Inference Engine Setup       ║"
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
            if [[ $(echo "$macos_version >= 12.0" | bc 2>/dev/null || echo "0") -eq 1 ]]; then
                print_success "macOS version supported"
            else
                print_warning "macOS 12.0+ recommended for best performance"
            fi
            ;;
        msys*|cygwin*)
            print_warning "Windows detected. Make sure you're running in WSL2 for best performance."
            OS_TYPE="windows"
            ;;
        *)
            print_error "Unsupported OS: $OSTYPE"
            echo "Supported platforms: Linux (NVIDIA GPU), macOS (Apple Silicon/Intel), Windows (WSL2)"
            exit 1
            ;;
    esac
    
    # Check Python
    if ! command -v python3 &> /dev/null; then
        print_error "Python 3 is required but not installed."
        echo "Please install Python 3.9-3.12 and try again."
        exit 1
    fi
    
    # Check GPU/Hardware
    if [[ "$OS_TYPE" == "linux" ]]; then
        if command -v nvidia-smi &> /dev/null; then
            nvidia-smi &> /dev/null
            if [ $? -eq 0 ]; then
                print_success "NVIDIA GPU detected"
                nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits
                GPU_TYPE="nvidia"
            else
                print_warning "nvidia-smi failed. GPU may not be available."
                GPU_TYPE="cpu"
            fi
        else
            print_warning "nvidia-smi not found. Using CPU backend."
            GPU_TYPE="cpu"
        fi
    elif [[ "$OS_TYPE" == "macos" ]]; then
        # Check for Apple Silicon
        if [[ $(uname -m) == "arm64" ]]; then
            print_success "Apple Silicon (M1/M2/M3) detected - Metal Performance Shaders will be used"
            GPU_TYPE="metal"
        else
            print_success "Intel Mac detected - CPU backend will be used"
            GPU_TYPE="cpu"
        fi
    else
        GPU_TYPE="cpu"
    fi
    
    # Check available disk space (need ~20GB for models)
    available_space=$(df / | awk 'NR==2 {print $4}')
    if [ $available_space -lt 20971520 ]; then  # 20GB in KB
        print_warning "Less than 20GB available disk space. Large models may not fit."
    fi
}

setup_conda_environment() {
    print_step "Setting up Python environment..."
    
    # Check Python version compatibility
    python_version=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
    print_step "Detected Python version: $python_version"
    
    # Recommend Python 3.11 for macOS for better compatibility
    if [[ "$OS_TYPE" == "macos" && "$python_version" == "3.12" ]]; then
        print_warning "Python 3.12 detected on macOS. Python 3.11 is recommended for better vLLM compatibility."
        print_step "You may want to use: conda create -n vllm python=3.11 -y"
    fi
    
    if command -v conda &> /dev/null; then
        print_success "Conda found, creating environment..."
        
        # Use Python 3.11 for macOS by default
        if [[ "$OS_TYPE" == "macos" ]]; then
            PYTHON_VERSION="3.11"
            print_step "Using Python 3.11 for better macOS compatibility"
        fi
        
        # Create conda environment
        conda create -n $CONDA_ENV_NAME python=$PYTHON_VERSION -y
        
        # Activate environment
        source "$(conda info --base)/etc/profile.d/conda.sh"
        conda activate $CONDA_ENV_NAME
        
        print_success "Conda environment '$CONDA_ENV_NAME' created and activated"
    else
        print_warning "Conda not found. Using system Python with virtual environment..."
        
        # Create virtual environment
        python3 -m venv vllm_env
        source vllm_env/bin/activate
        
        print_success "Virtual environment created and activated"
    fi
}

install_vllm() {
    print_step "Installing vLLM..."
    
    # Upgrade pip
    pip install --upgrade pip
    
    # Install based on OS and hardware
    if [[ "$OS_TYPE" == "linux" && "$GPU_TYPE" == "nvidia" ]]; then
        print_step "Installing vLLM with CUDA 12.8 support..."
        pip install vllm --extra-index-url https://download.pytorch.org/whl/cu128
    elif [[ "$OS_TYPE" == "macos" ]]; then
        print_step "Installing vLLM for macOS..."
        
        # Install required dependencies for macOS
        print_step "Installing macOS dependencies..."
        
        # Check if Homebrew is available
        if ! command -v brew &> /dev/null; then
            print_error "Homebrew is required for macOS installation"
            echo "Please install Homebrew first: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            exit 1
        fi
        
        # Install system dependencies
        print_step "Installing system dependencies via Homebrew..."
        brew install cmake ninja
        
        # Ensure Xcode Command Line Tools are installed
        if ! xcode-select -p &> /dev/null; then
            print_step "Installing Xcode Command Line Tools..."
            xcode-select --install
            print_warning "Please complete Xcode Command Line Tools installation and re-run this script"
            exit 1
        fi
        
        # Install required build dependencies first
        print_step "Installing build dependencies..."
        pip install --upgrade pip setuptools wheel setuptools-scm
        pip install regex packaging ninja pybind11 cmake
        
        # Install PyTorch with Metal support for Apple Silicon
        if [[ "$GPU_TYPE" == "metal" ]]; then
            print_step "Installing PyTorch with Metal Performance Shaders support..."
            pip install torch torchvision torchaudio
        else
            print_step "Installing PyTorch for Intel Mac..."
            pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
        fi
        
        # Install additional dependencies needed for vLLM build
        print_step "Installing additional build dependencies..."
        pip install transformers tokenizers datasets accelerate
        pip install numpy psutil ray typing-extensions
        
        # Install dependencies that commonly cause build issues
        pip install protobuf sentencepiece
        
        # Install vLLM from source for macOS
        print_step "Installing vLLM from source (this may take 10-15 minutes)..."
        
        # Set environment variables for macOS build
        export MACOSX_DEPLOYMENT_TARGET=12.0
        if [[ "$GPU_TYPE" == "metal" ]]; then
            export VLLM_TARGET_DEVICE=cpu  # Use CPU backend for now, Metal support is experimental
        else
            export VLLM_TARGET_DEVICE=cpu
        fi
        
        # Install vLLM with proper dependency resolution
        pip install --upgrade pip setuptools wheel
        
        # macOS has known version conflicts with vLLM, so provide clear guidance
        print_error "macOS vLLM installation is not recommended due to known issues."
        echo ""
        echo -e "${RED}❌ vLLM on macOS Issues:${NC}"
        echo "• Version metadata conflicts (infinite pip loops)"
        echo "• No GPU acceleration (CPU only)"
        echo "• Complex build dependencies"
        echo "• 5-10x slower performance"
        echo "• Frequent installation failures"
        echo ""
        echo -e "${GREEN}✅ Recommended Solution: Use Ollama${NC}"
        echo ""
        echo "Ollama advantages on macOS:"
        echo "🚀 Native Apple Silicon optimization"
        echo "⚡ 2-3x better performance than vLLM"
        echo "🛠️ Easy installation and setup"
        echo "🔧 No version conflicts or build issues"
        echo "📦 Same models and API compatibility"
        echo ""
        read -p "Switch to Ollama setup now? (Y/n): " choice
        if [[ ! "$choice" =~ ^[Nn]$ ]]; then
            print_step "Switching to Ollama setup (recommended)..."
            cd .. && exec ./setup_ollama.sh
            exit 0
        fi
        
        print_warning "Proceeding with vLLM installation (not recommended)..."
        print_step "Trying specific vLLM version known to work on some macOS systems..."
        
        # Try a specific older version that might work
        if pip install "vllm==0.6.2" --no-build-isolation --timeout=300; then
            print_success "Installed vLLM 0.6.2"
        else
            print_error "vLLM installation failed. Please use Ollama instead."
            echo "Run: cd .. && ./setup_ollama.sh"
            exit 1
        fi
        
    else
        print_step "Installing vLLM with CPU support..."
        export VLLM_TARGET_DEVICE=cpu
        pip install vllm --no-build-isolation
    fi
    
    # Verify installation
    python -c "import vllm; print('vLLM version:', vllm.__version__)" 2>/dev/null
    if [ $? -eq 0 ]; then
        print_success "vLLM installed successfully"
        
        # Show platform-specific information
        if [[ "$OS_TYPE" == "macos" ]]; then
            print_step "macOS-specific notes:"
            echo "• vLLM on macOS uses CPU backend for maximum compatibility"
            echo "• Performance will be lower than Linux with NVIDIA GPU"
            echo "• Apple Silicon Macs will have better performance than Intel Macs"
            if [[ "$GPU_TYPE" == "metal" ]]; then
                echo "• Metal Performance Shaders acceleration is experimental"
            fi
        fi
    else
        print_error "vLLM installation failed"
        
        if [[ "$OS_TYPE" == "macos" ]]; then
            echo ""
            echo -e "${YELLOW}Common macOS installation issues and solutions:${NC}"
            echo ""
            echo "1. Missing build dependencies:"
            echo "   pip install regex packaging ninja pybind11 setuptools-scm"
            echo ""
            echo "2. Missing Xcode Command Line Tools:"
            echo "   xcode-select --install"
            echo "   sudo xcode-select --reset"
            echo ""
            echo "3. Missing Homebrew:"
            echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            echo "   brew install cmake ninja"
            echo ""
            echo "4. Python version compatibility:"
            echo "   conda create -n vllm python=3.11 -y  # Use 3.11 instead of 3.12"
            echo ""
            echo "5. Environment conflicts:"
            echo "   conda deactivate && conda activate vllm  # Reset environment"
            echo ""
            echo -e "${BLUE}Alternative: Ollama provides excellent macOS performance with easier setup${NC}"
            echo "   ./setup_ollama.sh"
        fi
        exit 1
    fi
}

show_model_menu() {
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${RED}⚠️  WARNING: vLLM has significant issues on macOS ⚠️${NC}"
        echo ""
        echo -e "${YELLOW}Known macOS problems:${NC}"
        echo "• Version conflicts causing infinite installation loops"
        echo "• No GPU acceleration (CPU only)"
        echo "• Complex build dependencies"
        echo "• Performance 5-10x slower than Linux"
        echo ""
        echo -e "${GREEN}RECOMMENDED: Use Ollama instead for macOS${NC}"
        echo "• Native Apple Silicon optimization"
        echo "• Easy installation"
        echo "• Better performance"
        echo ""
        read -p "Continue with vLLM anyway? (y/N): " choice
        if [[ ! "$choice" =~ ^[Yy]$ ]]; then
            echo ""
            echo "Switching to Ollama setup (recommended for macOS)..."
            cd .. && exec ./setup_ollama.sh
            exit 0
        fi
        echo ""
    fi
    
    echo -e "${BLUE}Available models to install:${NC}"
    
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${YELLOW}macOS optimized selection (CPU backend):${NC}"
        echo "1) lightweight - $(get_model lightweight) ($(get_model_vram "$(get_model lightweight)") RAM) - Fast development/testing ⭐ Recommended"
        echo "2) chat - $(get_model chat) ($(get_model_vram "$(get_model chat)") RAM) - High-quality chat (slower on CPU)"
        echo "3) code - $(get_model code) ($(get_model_vram "$(get_model code)") RAM) - Code generation (slower on CPU)"
        echo "4) embeddings - $(get_model embeddings) ($(get_model_vram "$(get_model embeddings)") RAM) - Text embeddings ⭐ Recommended"
        echo "5) fast - $(get_model fast) ($(get_model_vram "$(get_model fast)") RAM) - Fast code generation ⭐ Recommended"
        echo "6) quality - $(get_model quality) ($(get_model_vram "$(get_model quality)") RAM) - Best quality (very slow on CPU)"
        echo "7) macos-recommended - Install macOS-optimized set (lightweight + embeddings + fast)"
        echo "8) custom - Enter custom model name"
        echo
        echo -e "${YELLOW}Note: On macOS, models use system RAM instead of GPU VRAM${NC}"
    else
        echo "1) lightweight - $(get_model lightweight) ($(get_model_vram "$(get_model lightweight)") VRAM) - Fast development/testing"
        echo "2) chat - $(get_model chat) ($(get_model_vram "$(get_model chat)") VRAM) - High-quality chat"
        echo "3) code - $(get_model code) ($(get_model_vram "$(get_model code)") VRAM) - Code generation"
        echo "4) embeddings - $(get_model embeddings) ($(get_model_vram "$(get_model embeddings)") VRAM) - Text embeddings"
        echo "5) fast - $(get_model fast) ($(get_model_vram "$(get_model fast)") VRAM) - Fast code generation"
        echo "6) quality - $(get_model quality) ($(get_model_vram "$(get_model quality)") VRAM) - Best quality chat"
        echo "7) all - Install all models (requires 40GB+ VRAM)"
        echo "8) custom - Enter custom model name"
    fi
    echo
}

download_model() {
    local model_name=$1
    local model_desc=$2
    
    print_step "Pre-downloading model: $model_name"
    echo "Description: $model_desc"
    echo "VRAM requirement: $(get_model_vram "$model_name")"
    
    # Use Python to download the model
    python -c "
from transformers import AutoTokenizer, AutoModelForCausalLM
import torch

try:
    print('Downloading tokenizer...')
    tokenizer = AutoTokenizer.from_pretrained('$model_name')
    print('Downloading model...')
    model = AutoModelForCausalLM.from_pretrained(
        '$model_name',
        torch_dtype=torch.float16,
        low_cpu_mem_usage=True,
        device_map='auto' if torch.cuda.is_available() else 'cpu'
    )
    print('Model downloaded successfully!')
except Exception as e:
    print(f'Download failed: {e}')
    exit(1)
"
    
    if [ $? -eq 0 ]; then
        print_success "Model $model_name downloaded successfully"
    else
        print_error "Failed to download model $model_name"
        return 1
    fi
}

install_models() {
    show_model_menu
    
    read -p "Select models to install (1-8): " choice
    
    case $choice in
        1)
            download_model "$(get_model lightweight)" "Lightweight chat model for development"
            ;;
        2)
            download_model "$(get_model chat)" "High-quality chat model"
            ;;
        3)
            download_model "$(get_model code)" "Code generation model"
            ;;
        4)
            download_model "$(get_model embeddings)" "Text embeddings model"
            ;;
        5)
            download_model "$(get_model fast)" "Fast code generation model"
            ;;
        6)
            download_model "$(get_model quality)" "Highest quality chat model"
            ;;
        7)
            if [[ "$OS_TYPE" == "macos" ]]; then
                print_step "Installing macOS-recommended model set..."
                download_model "$(get_model lightweight)" "lightweight model (best for macOS)"
                download_model "$(get_model embeddings)" "embeddings model (works well on CPU)"
                download_model "$(get_model fast)" "fast model (optimized for CPU)"
            else
                print_step "Installing all models..."
                download_model "$(get_model lightweight)" "lightweight model"
                download_model "$(get_model chat)" "chat model"
                download_model "$(get_model code)" "code model"
                download_model "$(get_model embeddings)" "embeddings model"
                download_model "$(get_model fast)" "fast model"
                download_model "$(get_model quality)" "quality model"
            fi
            ;;
        8)
            read -p "Enter custom model name (e.g., microsoft/DialoGPT-large): " custom_model
            download_model "$custom_model" "Custom model"
            ;;
        *)
            print_error "Invalid choice. Skipping model installation."
            ;;
    esac
}

create_config_files() {
    print_step "Creating configuration files..."
    
    # Create vLLM configuration directory
    mkdir -p ~/.config/circuit-breaker
    
    # Create vLLM environment file
    cat > ~/.config/circuit-breaker/vllm.env << EOF
# Circuit Breaker vLLM Configuration
VLLM_BASE_URL=http://localhost:$VLLM_PORT
VLLM_DEFAULT_MODEL=$(get_model lightweight)
VLLM_API_KEY=
VLLM_GPU_MEMORY_UTILIZATION=$GPU_MEMORY_UTILIZATION
VLLM_TENSOR_PARALLEL_SIZE=1
VLLM_MAX_NUM_SEQS=256
VLLM_MAX_MODEL_LEN=4096
VLLM_VERIFY_SSL=true
VLLM_TIMEOUT_SECONDS=120
EOF
    
    # Create startup script
    cat > ~/.config/circuit-breaker/start_vllm.sh << 'EOF'
#!/bin/bash

# Circuit Breaker vLLM Startup Script

# Source configuration
source ~/.config/circuit-breaker/vllm.env

# Default model if not specified
MODEL=${1:-$VLLM_DEFAULT_MODEL}
PORT=${2:-8000}

echo "Starting vLLM server with model: $MODEL"
echo "Server will be available at: http://localhost:$PORT"

# Detect OS for platform-specific configuration
OS_TYPE=""
case "$OSTYPE" in
    linux-gnu*) OS_TYPE="linux" ;;
    darwin*) OS_TYPE="macos" ;;
    *) OS_TYPE="other" ;;
esac

# Start vLLM server with platform-specific settings
if [[ "$OS_TYPE" == "macos" ]]; then
    # macOS-specific settings
    echo "Starting vLLM on macOS with CPU backend..."
    vllm serve "$MODEL" \
        --host 0.0.0.0 \
        --port $PORT \
        --max-num-seqs $VLLM_MAX_NUM_SEQS \
        --max-model-len $VLLM_MAX_MODEL_LEN \
        --device cpu \
        --dtype float16
else
    # Linux/other settings
    vllm serve "$MODEL" \
        --host 0.0.0.0 \
        --port $PORT \
        --gpu-memory-utilization $VLLM_GPU_MEMORY_UTILIZATION \
        --max-num-seqs $VLLM_MAX_NUM_SEQS \
        --max-model-len $VLLM_MAX_MODEL_LEN \
        --tensor-parallel-size $VLLM_TENSOR_PARALLEL_SIZE
fi
EOF
    
    chmod +x ~/.config/circuit-breaker/start_vllm.sh
    
    print_success "Configuration files created in ~/.config/circuit-breaker/"
}

test_installation() {
    print_step "Testing vLLM installation..."
    
    # Test basic import
    python -c "import vllm; print('✓ vLLM import successful')"
    
    # Test model loading with smallest model
    lightweight_model=$(get_model lightweight)
    print_step "Testing model loading with $lightweight_model..."
    
    # Platform-specific testing
    if [[ "$OS_TYPE" == "macos" ]]; then
        print_step "Running macOS-optimized test (CPU backend)..."
        print_warning "This may take 2-3 minutes on macOS..."
        timeout 180 python -c "
from vllm import LLM
import sys

try:
    # macOS-specific settings
    llm = LLM('$lightweight_model', device='cpu', max_model_len=256, max_num_seqs=2)
    print('✓ Model loading successful on macOS')
    
    # Test generation with shorter context for CPU
    outputs = llm.generate(['Hello'], max_tokens=5)
    print('✓ Text generation successful')
    print('Sample output:', outputs[0].outputs[0].text[:30])
    
except Exception as e:
    print(f'✗ Test failed: {e}')
    print('Note: On macOS, some models may require more memory or time to load')
    sys.exit(1)
"
    else
        # Linux/GPU testing
        timeout 60 python -c "
from vllm import LLM
import sys

try:
    llm = LLM('$lightweight_model', gpu_memory_utilization=0.3, max_model_len=512)
    print('✓ Model loading successful')
    
    # Test generation
    outputs = llm.generate(['Hello'], max_tokens=10)
    print('✓ Text generation successful')
    print('Sample output:', outputs[0].outputs[0].text[:50])
    
except Exception as e:
    print(f'✗ Test failed: {e}')
    sys.exit(1)
"
    fi
    
    if [ $? -eq 0 ]; then
        print_success "vLLM installation test passed!"
    else
        print_warning "Installation test failed. Manual verification may be needed."
        if [[ "$OS_TYPE" == "macos" ]]; then
            echo ""
            echo -e "${YELLOW}macOS troubleshooting tips:${NC}"
            echo "• Try a smaller model: microsoft/DialoGPT-small"
            echo "• Ensure sufficient RAM (12GB+ recommended for testing)"
            echo "• Check Xcode Command Line Tools: xcode-select -p"
            echo "• Verify Python environment: which python && python --version"
            echo "• Test basic import: python -c 'import vllm; print(\"OK\")'"
            echo ""
            echo -e "${BLUE}Consider using Ollama for better macOS experience:${NC}"
            echo "  ./setup_ollama.sh"
        fi
    fi
}

create_examples() {
    print_step "Creating example scripts..."
    
    mkdir -p examples/vllm
    
    # Create basic example
    cat > examples/vllm/basic_chat.py << 'EOF'
#!/usr/bin/env python3
"""
Basic vLLM chat example for Circuit Breaker integration
"""

import requests
import json

def test_vllm_chat():
    url = "http://localhost:8000/v1/chat/completions"
    
    payload = {
        "model": "microsoft/DialoGPT-medium",
        "messages": [
            {"role": "user", "content": "Hello! How are you today?"}
        ],
        "max_tokens": 100,
        "temperature": 0.7
    }
    
    try:
        response = requests.post(url, json=payload)
        response.raise_for_status()
        
        result = response.json()
        print("Response:", result["choices"][0]["message"]["content"])
        print("Usage:", result["usage"])
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure vLLM server is running: ~/.config/circuit-breaker/start_vllm.sh")

if __name__ == "__main__":
    test_vllm_chat()
EOF

    # Create streaming example
    cat > examples/vllm/streaming_chat.py << 'EOF'
#!/usr/bin/env python3
"""
Streaming vLLM chat example for Circuit Breaker integration
"""

import requests
import json

def test_vllm_streaming():
    url = "http://localhost:8000/v1/chat/completions"
    
    payload = {
        "model": "microsoft/DialoGPT-medium",
        "messages": [
            {"role": "user", "content": "Write a short story about a robot learning to paint."}
        ],
        "max_tokens": 200,
        "temperature": 0.8,
        "stream": True
    }
    
    try:
        response = requests.post(url, json=payload, stream=True)
        response.raise_for_status()
        
        print("Streaming response:")
        for line in response.iter_lines():
            if line:
                line = line.decode('utf-8')
                if line.startswith('data: '):
                    data = line[6:]
                    if data == '[DONE]':
                        break
                    try:
                        chunk = json.loads(data)
                        content = chunk["choices"][0]["delta"].get("content", "")
                        if content:
                            print(content, end="", flush=True)
                    except json.JSONDecodeError:
                        continue
        print("\n")
        
    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        print("Make sure vLLM server is running: ~/.config/circuit-breaker/start_vllm.sh")

if __name__ == "__main__":
    test_vllm_streaming()
EOF

    chmod +x examples/vllm/*.py
    
    print_success "Example scripts created in examples/vllm/"
}

show_completion_message() {
    echo -e "${GREEN}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                    Setup Complete! 🎉                         ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    echo -e "${BLUE}Quick Start Commands:${NC}"
    echo
    echo "1. Start vLLM server:"
    echo "   ~/.config/circuit-breaker/start_vllm.sh"
    echo
    echo "2. Test with different models:"
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo "   ~/.config/circuit-breaker/start_vllm.sh $(get_model lightweight)  # Recommended for macOS"
        echo "   ~/.config/circuit-breaker/start_vllm.sh $(get_model fast)         # Also good for macOS"
    else
        echo "   ~/.config/circuit-breaker/start_vllm.sh $(get_model chat)"
        echo "   ~/.config/circuit-breaker/start_vllm.sh $(get_model code)"
    fi
    echo
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${YELLOW}macOS-specific notes:${NC}"
        echo "• vLLM uses CPU backend on macOS (no GPU acceleration)"
        echo "• Performance is 5-10x slower than Linux GPU"
        echo "• May have compatibility issues with newer Python versions"
        echo "• Limited model support compared to Linux"
        echo ""
        echo -e "${GREEN}For better macOS experience, consider Ollama:${NC}"
        echo "• Native Apple Silicon optimization"
        echo "• 2-3x better performance than vLLM on macOS"
        echo "• Easier setup and management"
        echo "• Run: ./setup_ollama.sh"
        echo
    fi
    echo "3. Test the integration:"
    echo "   python examples/vllm/basic_chat.py"
    echo "   python examples/vllm/streaming_chat.py"
    echo
    echo "4. Start Circuit Breaker server:"
    echo "   cd circuit-breaker && cargo run --bin server"
    echo
    echo "5. Test Circuit Breaker with vLLM:"
    echo "   curl http://localhost:3000/v1/chat/completions \\"
    echo "     -H 'Content-Type: application/json' \\"
    echo "     -d '{\"model\": \"vllm://microsoft/DialoGPT-medium\", \"messages\": [{\"role\": \"user\", \"content\": \"Hello!\"}]}'"
    echo
    echo -e "${YELLOW}Configuration files:${NC}"
    echo "• Environment: ~/.config/circuit-breaker/vllm.env"
    echo "• Startup script: ~/.config/circuit-breaker/start_vllm.sh"
    echo "• Examples: examples/vllm/"
    echo
    if [[ "$OS_TYPE" == "macos" ]]; then
        echo -e "${GREEN}Better alternative for macOS users:${NC}"
        echo "🚀 ./setup_ollama.sh"
        echo ""
        echo "Ollama advantages on macOS:"
        echo "✅ Native Apple Silicon optimization"
        echo "✅ 2-3x better performance than vLLM"
        echo "✅ Easier setup and management"
        echo "✅ No version conflicts or build issues"
        echo "✅ Same models and API compatibility"
        echo
    fi
    echo -e "${BLUE}📚 Documentation: docs/VLLM_INTEGRATION.md${NC}"
}

main() {
    print_header
    
    # Check if help requested
    if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
        echo "Circuit Breaker vLLM Setup Script"
        echo
        echo "Usage: $0 [options]"
        echo
        echo "Options:"
        echo "  --help, -h              Show this help message"
        echo "  --skip-models          Skip model installation"
        echo "  --skip-test            Skip installation test"
        echo "  --port PORT            Set vLLM server port (default: 8000)"
        echo "  --gpu-memory FLOAT     Set GPU memory utilization (default: 0.9)"
        echo
        echo "Environment Variables:"
        echo "  VLLM_PORT              Server port (default: 8000)"
        echo "  GPU_MEMORY_UTILIZATION GPU memory utilization (default: 0.9)"
        echo "  CONDA_ENV_NAME         Conda environment name (default: vllm)"
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
                VLLM_PORT="$2"
                shift 2
                ;;
            --gpu-memory)
                GPU_MEMORY_UTILIZATION="$2"
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
    setup_conda_environment
    install_vllm
    
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