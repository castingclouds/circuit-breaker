#!/bin/bash

# Test script to check if Ollama models are available via OpenAI API endpoint
# This script starts the server and tests the models endpoint

set -e

echo "🧪 Testing Circuit Breaker Models Endpoint"
echo "=========================================="

# Check if Ollama is running
echo "🔍 Checking if Ollama is running..."
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "❌ Ollama is not running. Please start it with: ollama serve"
    exit 1
fi
echo "✅ Ollama is running"

# Set environment variables
export OLLAMA_BASE_URL="http://localhost:11434"
export LOG_LEVEL="info"

echo ""
echo "🚀 Starting Circuit Breaker server..."

# Start the server in background
OLLAMA_BASE_URL=http://localhost:11434 cargo run --bin server &
SERVER_PID=$!

# Function to cleanup server on exit
cleanup() {
    echo ""
    echo "🛑 Stopping server..."
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 8

# Test health endpoint
echo ""
echo "🏥 Testing health endpoint..."
if curl -s http://localhost:3000/health > /dev/null; then
    echo "✅ Health endpoint is responding"
else
    echo "❌ Health endpoint is not responding"
    exit 1
fi

# Test models endpoint
echo ""
echo "📋 Testing models endpoint..."
MODELS_RESPONSE=$(curl -s http://localhost:3000/v1/models)

if [ $? -eq 0 ]; then
    echo "✅ Models endpoint is responding"
    
    # Parse response and count models
    TOTAL_MODELS=$(echo "$MODELS_RESPONSE" | jq '.data | length' 2>/dev/null || echo "0")
    echo "📊 Total models found: $TOTAL_MODELS"
    
    # Check for Ollama models specifically
    OLLAMA_MODELS=$(echo "$MODELS_RESPONSE" | jq '.data[] | select(.extra.provider == "Ollama") | .id' 2>/dev/null || echo "")
    
    if [ -n "$OLLAMA_MODELS" ]; then
        echo "✅ Ollama models found:"
        echo "$OLLAMA_MODELS" | sed 's/"//g' | sed 's/^/  - /'
    else
        echo "⚠️  No Ollama models found in the response"
        echo "🔍 Available providers:"
        echo "$MODELS_RESPONSE" | jq '.data[].extra.provider' 2>/dev/null | sort | uniq | sed 's/"//g' | sed 's/^/  - /' || echo "  - Unable to parse providers"
    fi
    
    # Show full response for debugging
    echo ""
    echo "🔍 Full models response:"
    echo "$MODELS_RESPONSE" | jq '.' 2>/dev/null || echo "$MODELS_RESPONSE"
    
else
    echo "❌ Models endpoint is not responding"
    exit 1
fi

echo ""
echo "🎉 Test completed!"

# Keep server running for manual testing
echo ""
echo "💡 Server is still running for manual testing:"
echo "   Health: http://localhost:3000/health"
echo "   Models: http://localhost:3000/v1/models"
echo "   GraphQL: http://localhost:4000"
echo ""
echo "Press Ctrl+C to stop the server..."

# Wait for user to stop
wait $SERVER_PID