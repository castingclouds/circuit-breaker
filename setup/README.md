# Circuit Breaker Setup Scripts

This directory contains automated setup scripts for installing and configuring local AI providers with Circuit Breaker.

## 🚀 Quick Start

### Ollama Setup (Recommended for Beginners)
```bash
# Run the Ollama setup script
./setup_ollama.sh

# Follow the prompts to install models
# Recommended: Choose option 5 (recommended set)
```

### vLLM Setup (High Performance)
```bash
# Local installation (Linux with GPU)
./setup_vllm.sh

# Follow the prompts to install models
# Recommended: Start with option 1 (lightweight)

# For macOS users experiencing vLLM issues:
./macos_vllm_alternative.sh

# Cloud installation (EC2 with GPU) - Recommended for macOS users
./setup_vllm_ec2.sh
```

## 📋 What These Scripts Do

### `setup_ollama.sh`
- ✅ **Installs Ollama** on Linux, macOS, and Windows (WSL)
- ✅ **Starts Ollama service** automatically
- ✅ **Downloads models** based on Circuit Breaker provider definitions
- ✅ **Creates configuration files** for easy management
- ✅ **Tests installation** to ensure everything works
- ✅ **Provides example scripts** for testing

**Models Available:**
- `qwen2.5-coder:3b` (2GB) - Coding assistant
- `gemma2:2b` (1.6GB) - Fast general purpose
- `llama3.1:8b` (4.7GB) - High-quality chat
- `nomic-embed-text:latest` (274MB) - Text embeddings

### `setup_vllm.sh`
- ✅ **Installs vLLM** with CUDA support (Linux only)
- ✅ **Sets up Python environment** (conda or virtualenv)
- ✅ **Downloads HuggingFace models** optimized for vLLM
- ✅ **Creates startup scripts** for different models
- ✅ **Configures performance settings** (GPU memory, parallelism)
- ✅ **Tests high-performance inference**

### `setup_vllm_ec2.sh` (Recommended for macOS)
- ✅ **Launches AWS EC2 instance** with GPU acceleration
- ✅ **Automatically installs vLLM** on Ubuntu with CUDA
- ✅ **Configures remote access** and security groups
- ✅ **Sets up local integration** with Circuit Breaker
- ✅ **Provides management tools** for start/stop/monitoring
- ✅ **Cost-effective** (~$0.50/hour for development)

**Models Available:**
- `microsoft/DialoGPT-medium` (2GB) - Lightweight chat
- `meta-llama/Llama-2-7b-chat-hf` (8GB) - Production chat
- `codellama/CodeLlama-7b-Instruct-hf` (8GB) - Code generation
- `sentence-transformers/all-MiniLM-L6-v2` (512MB) - Embeddings

## 🎯 Which Should You Choose?

| Feature | Ollama | vLLM (Linux GPU) | vLLM (EC2 GPU) | vLLM (macOS CPU) | Recommendation |
|---------|--------|------------------|----------------|------------------|----------------|
| **Setup Difficulty** | Easy | Medium | Easy | Hard | Ollama or EC2 for beginners |
| **Performance** | Good | Excellent | Excellent | Poor | vLLM EC2 for production |
| **Memory Usage** | Standard | GPU Optimized | GPU Optimized | RAM-based | vLLM for limited local VRAM |
| **Model Selection** | Curated | Any HuggingFace | Any HuggingFace | Any HuggingFace | vLLM for model variety |
| **Throughput** | ~50 req/s | ~500+ req/s | ~500+ req/s | ~10-30 req/s | vLLM EC2 for high load |
| **Cost** | Free | Hardware cost | ~$0.50/hour | Free | Ollama for free, EC2 for performance |
| **macOS Experience** | Native & Optimized | N/A | Remote Access | CPU Backend | Ollama or EC2 recommended |

## 📖 Usage Instructions

### Basic Usage
```bash
# Make scripts executable (if not already)
chmod +x setup_ollama.sh setup_vllm.sh

# Run with default settings
./setup_ollama.sh
./setup_vllm.sh
```

### Advanced Options

**Ollama Options:**
```bash
./setup_ollama.sh --help                 # Show help
./setup_ollama.sh --skip-models         # Install Ollama but skip models
./setup_ollama.sh --skip-test           # Skip installation test
./setup_ollama.sh --port 11435          # Use custom port
```

**vLLM Options:**
```bash
./setup_vllm.sh --help                  # Show help (local Linux installation)
./setup_vllm.sh --skip-models          # Install vLLM but skip models
./setup_vllm.sh --skip-test            # Skip installation test
./setup_vllm.sh --port 8001            # Use custom port
./setup_vllm.sh --gpu-memory 0.8       # Set GPU memory utilization (Linux only)

# EC2 vLLM Options:
./setup_vllm_ec2.sh --help             # Show help (EC2 cloud installation)
./setup_vllm_ec2.sh --instance-type g4dn.xlarge  # Specify instance type
./setup_vllm_ec2.sh --model codellama/CodeLlama-7b-Instruct-hf  # Choose model
```

## 🔧 After Installation

### Start Services

**Ollama:**
```bash
# Ollama starts automatically, or manually:
ollama serve

# Chat with a model:
ollama run qwen2.5-coder:3b

# Manage models:
~/.config/circuit-breaker/manage_ollama.sh list
```

**vLLM:**
```bash
# Local vLLM server:
~/.config/circuit-breaker/start_vllm.sh

# Start with specific model:
~/.config/circuit-breaker/start_vllm.sh meta-llama/Llama-2-7b-chat-hf  # Linux GPU
~/.config/circuit-breaker/start_vllm.sh microsoft/DialoGPT-medium     # macOS (not recommended)

# EC2 vLLM management:
~/.config/circuit-breaker/manage_vllm_ec2.sh status    # Check status
~/.config/circuit-breaker/manage_vllm_ec2.sh start     # Start EC2 instance
~/.config/circuit-breaker/manage_vllm_ec2.sh stop      # Stop EC2 instance
~/.config/circuit-breaker/manage_vllm_ec2.sh ssh       # SSH into instance
```

### Test Integration with Circuit Breaker

1. **Start Circuit Breaker server:**
   ```bash
   cd circuit-breaker
   cargo run --bin server
   ```

2. **Test Ollama integration:**
   ```bash
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "ollama://qwen2.5-coder:3b",
       "messages": [{"role": "user", "content": "Hello!"}]
     }'
   ```

3. **Test vLLM integration:**
   ```bash
   # Local vLLM
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "vllm://microsoft/DialoGPT-medium",
       "messages": [{"role": "user", "content": "Hello!"}]
     }'
   
   # EC2 vLLM (automatically configured)
   source ~/.config/circuit-breaker/vllm_ec2.env
   curl http://localhost:3000/v1/chat/completions \
     -H "Content-Type: application/json" \
     -d '{
       "model": "vllm://meta-llama/Llama-2-7b-chat-hf",
       "messages": [{"role": "user", "content": "Hello from EC2!"}]
     }'
   ```

## 📁 Files Created

After running the setup scripts, you'll find:

### Configuration Files
- `~/.config/circuit-breaker/ollama.env` - Ollama environment variables
- `~/.config/circuit-breaker/vllm.env` - vLLM environment variables
- `~/.config/circuit-breaker/manage_ollama.sh` - Ollama management script
- `~/.config/circuit-breaker/start_vllm.sh` - vLLM startup script

### Example Scripts
- `examples/ollama/basic_chat.py` - Basic Ollama testing
- `examples/ollama/streaming_chat.py` - Streaming and embeddings
- `examples/vllm/basic_chat.py` - Basic vLLM testing
- `examples/vllm/streaming_chat.py` - vLLM streaming example
- `setup_vllm_ec2.sh` - EC2 GPU setup for high performance
- `manage_vllm_ec2.sh` - EC2 instance management (auto-created)

## 🛠️ Troubleshooting

### Common Issues

**Ollama not starting:**
```bash
# Check if port is in use
netstat -tlnp | grep 11434

# Restart Ollama service
pkill ollama
ollama serve
```

**vLLM CUDA errors (Linux):**
```bash
# Check NVIDIA drivers
nvidia-smi

# Verify CUDA installation
python -c "import torch; print(torch.cuda.is_available())"

# Reduce GPU memory usage
export VLLM_GPU_MEMORY_UTILIZATION=0.7
```

**vLLM macOS errors:**
```bash
# If vLLM gets stuck in infinite loop or fails to install:
./macos_vllm_alternative.sh  # Switches to Ollama automatically

# Manual troubleshooting:
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew if missing
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Use Python 3.11 for better compatibility
conda create -n vllm python=3.11 -y

# For memory issues, try smaller models
export VLLM_MAX_MODEL_LEN=1024
```

**Models not downloading:**
```bash
# Check disk space
df -h

# Check internet connection
curl -I https://huggingface.co

# Clear model cache if needed
rm -rf ~/.cache/huggingface
```

### Getting Help

1. **Check logs:**
   - Ollama: `~/.config/circuit-breaker/manage_ollama.sh logs`
   - vLLM: Check terminal output where server was started

2. **Test individual components:**
   - Run example scripts in `examples/` directory
   - Use `--help` flag on setup scripts

3. **Environment verification:**
   ```bash
   # Check installed models
   ollama list                              # For Ollama
   ls ~/.cache/huggingface/transformers     # For vLLM
   
   # Check services
   curl http://localhost:11434              # Ollama
   curl http://localhost:8000/health        # vLLM
   ```

## 🎯 Performance Tips

### Ollama Optimization
- Use smaller models for development (`gemma2:2b`)
- Use larger models for production (`llama3.1:8b`)
- Keep models loaded with `OLLAMA_KEEP_ALIVE=24h`

### vLLM Optimization

**Linux (GPU):**
- Set `VLLM_GPU_MEMORY_UTILIZATION=0.9` for maximum performance
- Use quantization for memory-constrained systems
- Enable tensor parallelism for large models on multiple GPUs

**macOS (CPU):**
- Use smaller models (`microsoft/DialoGPT-medium`)
- Set `VLLM_MAX_MODEL_LEN=1024` for better performance
- Limit concurrent sequences: `VLLM_MAX_NUM_SEQS=4`
- Use `dtype=float16` to reduce memory usage

## 📚 Additional Resources

- **[Ollama Documentation](https://github.com/ollama/ollama)**
- **[vLLM Documentation](https://docs.vllm.ai/)**
- **[Circuit Breaker Integration Guide](../docs/OPENROUTER_ALTERNATIVE.md)**
- **[Provider Configuration](../docs/AGENT_CONFIGURATION.md)**

---

**🚀 Ready to get started? Choose your setup script and run it!**

**For beginners (all platforms):** `./setup_ollama.sh`  
**For Linux performance:** `./setup_vllm.sh`  
**For cloud performance:** `./setup_vllm_ec2.sh` ⚡ **Recommended for serious workloads**
**For macOS users:** 
- `./setup_ollama.sh` - Native performance, easy setup ⭐ Recommended for local dev
- `./setup_vllm_ec2.sh` - Cloud GPU performance ⚡ Recommended for production
- `./macos_vllm_alternative.sh` - Quick solution if local vLLM fails 🆘

**macOS Note:** For the best vLLM experience on macOS, use EC2 with GPU acceleration (~$0.50/hour). Local vLLM has compatibility issues (version conflicts, infinite loops, poor performance). Ollama provides the best local macOS experience.
