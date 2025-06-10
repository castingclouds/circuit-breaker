#!/bin/bash

# Test script for Ollama provider with specific models
# This script tests the Ollama provider with the user's specific models

set -e

echo "🦙 Testing Ollama Provider with Specific Models"
echo "==============================================="

# Check if Ollama is running
echo "🔍 Checking if Ollama is running..."
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "❌ Ollama is not running. Please start it with: ollama serve"
    exit 1
fi
echo "✅ Ollama is running"

# Check available models
echo ""
echo "📋 Available models:"
ollama list

# Check if required models are available
echo ""
echo "🔍 Checking for required models..."

REQUIRED_MODELS=("qwen2.5-coder:3b" "gemma3:4b" "nomic-embed-text:latest")
MISSING_MODELS=()

for model in "${REQUIRED_MODELS[@]}"; do
    if ollama list | grep -q "$model"; then
        echo "✅ $model is available"
    else
        echo "❌ $model is missing"
        MISSING_MODELS+=("$model")
    fi
done

# Pull missing models if any
if [ ${#MISSING_MODELS[@]} -gt 0 ]; then
    echo ""
    echo "⬇️  Pulling missing models..."
    for model in "${MISSING_MODELS[@]}"; do
        echo "Pulling $model..."
        ollama pull "$model"
    done
fi

# Set environment variables for testing
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_DEFAULT_MODEL="qwen2.5-coder:3b"
export OLLAMA_EMBEDDING_MODEL="nomic-embed-text:latest"

echo ""
echo "🧪 Running Ollama provider tests..."

# Run the test
if cargo run --example ollama_provider_test; then
    echo ""
    echo "✅ Basic tests passed!"
else
    echo ""
    echo "❌ Basic tests failed!"
    exit 1
fi

# Test with streaming if user wants
echo ""
read -p "🌊 Test streaming responses? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🌊 Testing streaming responses..."
    TEST_STREAMING=true cargo run --example ollama_provider_test
fi

echo ""
echo "🎉 All tests completed!"
echo ""
echo "💡 Model recommendations based on your setup:"
echo "  📝 General chat: gemma3:4b"
echo "  💻 Code generation: qwen2.5-coder:3b"  
echo "  🔍 Text embeddings: nomic-embed-text:latest"
echo ""
echo "🚀 Your Ollama provider is ready for production use!"