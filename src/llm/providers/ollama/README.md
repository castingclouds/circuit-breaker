# Ollama Provider for Circuit Breaker

This module provides integration with [Ollama](https://ollama.ai/), allowing you to use local LLM models through Circuit Breaker's unified LLM provider interface.

## Overview

Ollama is a tool that makes it easy to run large language models locally. Unlike cloud-based providers, Ollama models run on your own hardware, providing:

- **Privacy**: No data leaves your machine
- **Cost**: No per-token charges for local inference
- **Speed**: Low latency for local models (hardware dependent)
- **Offline capability**: Works without internet connection

## Prerequisites

1. **Install Ollama**: Download and install from [ollama.ai](https://ollama.ai/)
2. **Start Ollama**: Run `ollama serve` to start the server
3. **Pull models**: Download models with `ollama pull <model-name>`

```bash
# Install Ollama (macOS example)
curl -fsSL https://ollama.ai/install.sh | sh

# Start Ollama server
ollama serve

# Pull some popular models
ollama pull llama2
ollama pull codellama
ollama pull mistral
```

## Quick Start

### Basic Usage

```rust
use circuit_breaker::llm::{
    LLMRequest, ChatMessage, MessageRole,
    providers::ollama::{create_client_from_env, OllamaClient, OllamaConfig},
    traits::LLMProviderClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client from environment variables
    let client = create_client_from_env()?;
    
    // Or create with custom configuration
    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        default_model: "llama2".to_string(),
        ..Default::default()
    };
    let client = OllamaClient::new(config);
    
    // Create a chat request
        let request = LLMRequest {
            model: "qwen2.5-coder:3b".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello! What can you help me with?".to_string(),
                ..Default::default()
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        ..Default::default()
    };
    
    // Send the request
    let response = client.chat_completion(&request, "").await?;
    println!("Response: {}", response.choices[0].message.content);
    
    Ok(())
}
```

### Using with Provider Registry

```rust
use circuit_breaker::llm::{
    providers::{create_default_registry, OllamaFactory},
    traits::ProviderFactory,
};

// Create registry with all providers including Ollama
let mut registry = create_default_registry();

// Or register Ollama separately
let mut registry = ProviderRegistry::new();
registry.register_factory(Box::new(OllamaFactory));

// Create Ollama provider
let config = OllamaFactory.default_config();
registry.create_provider(LLMProviderType::Ollama, &config)?;

// Use the provider
let provider = registry.get_provider(&LLMProviderType::Ollama).unwrap();
let response = provider.chat_completion(&request, "").await?;
```

## Configuration

### Environment Variables

- `OLLAMA_BASE_URL`: Base URL for Ollama API (default: `http://localhost:11434`)
- `OLLAMA_DEFAULT_MODEL`: Default model to use (default: `llama2`)
- `OLLAMA_KEEP_ALIVE`: How long to keep models in memory (default: `5m`)
- `OLLAMA_VERIFY_SSL`: Whether to verify SSL certificates (default: `true`)

### OllamaConfig Structure

```rust
pub struct OllamaConfig {
    /// Base URL for Ollama API
    pub base_url: String,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,
    /// Keep alive setting for models
    pub keep_alive: String,
}
```

## Supported Models

The provider includes default configurations for popular models:

### Text Generation
- **Gemma 2**: `gemma2:4b` (recommended for general text)
- **Llama 2**: `llama2`, `llama2:13b`, `llama2:70b`
- **Llama 3**: `llama3`, `llama3:70b`
- **Mistral**: `mistral`

### Code Generation
- **Qwen2.5 Coder**: `qwen2.5-coder:3b` (recommended for coding tasks)
- **Code Llama**: `codellama`, `codellama:13b`
- **Phi**: `phi`

### Embeddings
- **Nomic Embed**: `nomic-embed-text:latest` (text embeddings)

### Getting Available Models

```rust
// Get default models (predefined list)
let models = client.get_available_models();

// Fetch actual models from Ollama instance
let available_models = client.fetch_available_models().await?;
for model in available_models {
    println!("Model: {} ({})", model.name, model.id);
    println!("  Context: {} tokens", model.context_window);
    println!("  Capabilities: {:?}", model.capabilities);
}
```

## Embeddings Support

The Ollama provider supports text embeddings through dedicated embedding models:

```rust
use circuit_breaker::llm::{EmbeddingsRequest, EmbeddingsInput};

// Single text embedding
let request = EmbeddingsRequest {
    id: uuid::Uuid::new_v4(),
    model: "nomic-embed-text:latest".to_string(),
    input: EmbeddingsInput::Text("Your text to embed".to_string()),
    user: None,
    metadata: HashMap::new(),
};

let response = client.embeddings(&request, "").await?;
println!("Embedding dimensions: {}", response.data[0].embedding.len());

// Batch embeddings
let batch_request = EmbeddingsRequest {
    id: uuid::Uuid::new_v4(),
    model: "nomic-embed-text:latest".to_string(),
    input: EmbeddingsInput::TextArray(vec![
        "First text".to_string(),
        "Second text".to_string(),
        "Third text".to_string(),
    ]),
    user: None,
    metadata: HashMap::new(),
};

let batch_response = client.embeddings(&batch_request, "").await?;
for (i, embedding_data) in batch_response.data.iter().enumerate() {
    println!("Text {}: {} dimensions", i + 1, embedding_data.embedding.len());
}
```

## Streaming Support

```rust
use futures::StreamExt;

// Create streaming request
let mut request = /* your request */;
request.stream = Some(true);

// Get streaming response
let mut stream = client.chat_completion_stream(request, "".to_string()).await?;

// Process stream chunks
while let Some(chunk_result) = stream.next().await {
    match chunk_result {
        Ok(chunk) => {
            if let Some(choice) = chunk.choices.first() {
                print!("{}", choice.delta.content);
            }
        }
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

## Health Checks

```rust
// Check if Ollama is available
let is_healthy = client.health_check("").await?;
if is_healthy {
    println!("Ollama is running and accessible");
} else {
    println!("Ollama is not available");
}

// Check availability without creating a client
use circuit_breaker::llm::providers::ollama::check_availability;
let available = check_availability("http://localhost:11434").await;
```

## Model Management

### Checking Model Availability

```rust
// Check if a specific model is supported
if client.supports_model("llama2") {
    println!("llama2 is available");
}

// Get model information
use circuit_breaker::llm::providers::ollama::get_model_info;
if let Some(info) = get_model_info("codellama") {
    println!("Model: {}", info.name);
    println!("Context window: {}", info.context_window);
    println!("Supports streaming: {}", info.supports_streaming);
}
```

### Model Recommendations

```rust
use circuit_breaker::llm::providers::ollama::get_recommended_models;

let recommendations = get_recommended_models();

// Get models for specific use cases
let chat_models = recommendations.get("chat").unwrap();
let code_models = recommendations.get("code").unwrap();
let reasoning_models = recommendations.get("reasoning").unwrap();
let embedding_models = recommendations.get("embeddings").unwrap();
```

## Error Handling

Common error scenarios and how to handle them:

```rust
match client.chat_completion(&request, "").await {
    Ok(response) => {
        // Handle successful response
        println!("Response: {}", response.choices[0].message.content);
    }
    Err(e) => {
        match e {
            LLMError::Network(_) => {
                eprintln!("Network error - is Ollama running?");
            }
            LLMError::Provider(msg) => {
                if msg.contains("model") && msg.contains("not found") {
                    eprintln!("Model not found - try 'ollama pull <model-name>'");
                } else {
                    eprintln!("Provider error: {}", msg);
                }
            }
            LLMError::Timeout(_) => {
                eprintln!("Request timed out - model might be loading");
            }
            _ => eprintln!("Other error: {}", e),
        }
    }
}
```

## Performance Considerations

### Hardware Requirements
- **RAM**: Models require significant memory (7B models ~4GB, 13B ~8GB, 70B ~40GB)
- **CPU**: More cores = faster inference (GPU support varies by model)
- **Storage**: Models range from 2GB to 40GB+ depending on size

### Optimization Tips
1. **Keep models loaded**: Use `keep_alive` setting to avoid reload delays
2. **Choose appropriate model size**: Balance quality vs. speed/memory
3. **Adjust context window**: Smaller contexts = faster inference
4. **Use quantized models**: Trade some quality for speed and memory

### Model Loading
```rust
// Models are loaded on first use and stay in memory based on keep_alive setting
let config = OllamaConfig {
    keep_alive: "10m".to_string(), // Keep in memory for 10 minutes
    ..Default::default()
};
```

## Integration with Circuit Breaker Features

### Cost Tracking
Local inference with Ollama is free, so cost tracking will show $0.00:

```rust
let response = client.chat_completion(&request, "").await?;
println!("Cost: ${:.6}", response.usage.estimated_cost); // Always 0.0
println!("Tokens used: {}", response.usage.total_tokens);
```

### Routing and Fallbacks
Use Ollama as a primary or fallback provider in routing strategies:

```rust
// Use Ollama for cost optimization (free local inference)
let strategy = RoutingStrategy::CostOptimized;

// Or use as fallback when cloud providers fail
let strategy = RoutingStrategy::FailoverChain;
```

## Troubleshooting

### Common Issues

1. **"Connection refused"**
   ```bash
   # Check if Ollama is running
   ollama serve
   ```

2. **"Model not found"**
   ```bash
   # Pull the model first
   ollama pull qwen2.5-coder:3b  # For code generation
   ollama pull gemma2:4b         # For general text
   ollama pull nomic-embed-text  # For embeddings
   
   # Check available models
   ollama list
   ```

3. **"Out of memory"**
   - Try a smaller model (e.g., `qwen2.5-coder:3b` instead of `llama3:70b`)
   - Close other applications
   - Increase system swap space

4. **"Embeddings not working"**
   - Ensure you're using an embedding model (e.g., `nomic-embed-text:latest`)
   - Check model supports embeddings: `ollama show nomic-embed-text`

4. **Slow responses**
   - Use smaller models for faster inference
   - Ensure models stay loaded with appropriate `keep_alive`
   - Reduce context window size

### Debug Mode
Enable debug logging to see detailed request/response information:

```rust
// Set environment variable
std::env::set_var("RUST_LOG", "debug");
tracing_subscriber::fmt::init();
```

## Examples

See the `examples/rust/ollama_provider_test.rs` file for a comprehensive example that demonstrates:

- Health checking
- Model discovery
- Chat completion
- Streaming responses
- Error handling

Run the example with:
```bash
# Basic test
cargo run --example ollama_provider_test

# Test with specific models
OLLAMA_DEFAULT_MODEL=qwen2.5-coder:3b \
OLLAMA_EMBEDDING_MODEL=nomic-embed-text:latest \
cargo run --example ollama_provider_test

# Test with streaming
TEST_STREAMING=true cargo run --example ollama_provider_test
```

## API Reference

For detailed API documentation, see the module documentation:

- [`OllamaClient`](client.rs) - Main client implementation
- [`OllamaConfig`](config.rs) - Configuration options
- [`OllamaRequest/Response`](types.rs) - Request/response types

## Contributing

When adding new features to the Ollama provider:

1. Follow the existing pattern used by other providers
2. Implement the `LLMProviderClient` trait completely
3. Add tests for new functionality
4. Update this README if adding new capabilities
5. Consider Ollama-specific features (like model pulling, multimodal support)

## Model-Specific Features

### Qwen2.5 Coder (qwen2.5-coder:3b)
- **Best for**: Code generation, code completion, debugging
- **Context window**: 32,768 tokens
- **Strengths**: Excellent at multiple programming languages
- **Use cases**: Code review, refactoring, explanation

### Gemma 2 (gemma2:4b)
- **Best for**: General text generation, conversation
- **Context window**: 8,192 tokens  
- **Strengths**: Good reasoning and following instructions
- **Use cases**: Writing, summarization, Q&A

### Nomic Embed Text (nomic-embed-text:latest)
- **Best for**: Text embeddings for similarity search
- **Output**: Dense vector representations
- **Strengths**: High-quality embeddings for retrieval
- **Use cases**: Semantic search, clustering, classification

## License

This module is part of Circuit Breaker and follows the same license terms.