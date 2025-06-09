# Multi-Provider LLM Implementation

This document describes the implementation of multi-provider LLM support in Circuit Breaker, extending the existing Anthropic-only pattern to support OpenAI and Google Gemini providers.

## Architecture Overview

The Circuit Breaker LLM system follows a layered architecture that separates provider-specific implementations from the unified routing layer:

```
┌─────────────────────────────────────────────────────────────┐
│                    GraphQL API Layer                        │
├─────────────────────────────────────────────────────────────┤
│                    LLM Router                               │
├─────────────────────────────────────────────────────────────┤
│  OpenAI Provider  │  Anthropic Provider  │  Google Provider │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Pattern

### 1. Provider Client Trait

All providers implement the `LLMProviderClient` trait, which defines the standard interface:

```rust
#[async_trait]
pub trait LLMProviderClient: Send + Sync {
    async fn chat_completion(&self, request: &LLMRequest, api_key: &str) -> LLMResult<LLMResponse>;
    async fn chat_completion_stream<'a>(&'a self, request: &'a LLMRequest, api_key: &'a str) -> LLMResult<Box<dyn Stream<Item = LLMResult<StreamingChunk>> + Send + Unpin + 'a>>;
    fn provider_type(&self) -> LLMProviderType;
    async fn health_check(&self, api_key: &str) -> LLMResult<bool>;
}
```

### 2. Provider Implementations

#### OpenAI Provider (`OpenAIProvider`)

- **Endpoint**: `https://api.openai.com/v1/chat/completions`
- **Authentication**: Bearer token in Authorization header
- **Models**: GPT-4, GPT-4o, GPT-3.5 Turbo, GPT-4 Turbo
- **Features**: Streaming, function calling, vision (GPT-4o)
- **Cost**: Variable by model (GPT-3.5 Turbo is most cost-effective)

#### Google Gemini Provider (`GoogleProvider`)

- **Endpoint**: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- **Authentication**: API key as query parameter
- **Models**: Gemini Pro, Gemini 1.5 Pro, Gemini 1.5 Flash
- **Features**: Very large context windows (up to 2M tokens), competitive pricing
- **Special Notes**: No system role (converted to user), different request/response format

#### Anthropic Provider (`AnthropicProvider`)

- **Endpoint**: `https://api.anthropic.com/v1/messages`
- **Authentication**: x-api-key header
- **Models**: Claude 3 Haiku, Claude 3 Sonnet, Claude 4 Sonnet
- **Features**: Large context windows, high-quality reasoning

### 3. Provider Factory

The `create_provider_client` function acts as a factory:

```rust
pub fn create_provider_client(
    provider_type: &LLMProviderType,
    base_url: Option<String>,
) -> Box<dyn LLMProviderClient> {
    match provider_type {
        LLMProviderType::OpenAI => Box::new(OpenAIProvider::new(base_url)),
        LLMProviderType::Anthropic => Box::new(AnthropicProvider::new(base_url)),
        LLMProviderType::Google => Box::new(GoogleProvider::new(base_url)),
        _ => Box::new(OpenAIProvider::new(base_url)), // Default fallback
    }
}
```

## Configuration

### Default Model Configurations

The system includes pre-configured models with accurate pricing and capabilities:

#### OpenAI Models
- **GPT-4o**: $0.000005/input token, $0.000015/output token, 128K context, vision
- **GPT-4**: $0.00003/input token, $0.00006/output token, 8K context
- **GPT-3.5 Turbo**: $0.000001/input token, $0.000002/output token, 16K context

#### Google Gemini Models
- **Gemini 1.5 Pro**: $0.0000035/input token, $0.0000105/output token, 2M context
- **Gemini 1.5 Flash**: $0.000000075/input token, $0.0000003/output token, 1M context
- **Gemini Pro**: $0.0000005/input token, $0.0000015/output token, 32K context

#### Anthropic Models
- **Claude 3.5 Sonnet**: $0.000003/input token, $0.000015/output token, 200K context
- **Claude 3 Haiku**: $0.00000025/input token, $0.00000125/output token, 200K context

### Environment Variables

The system requires provider-specific API keys:

```bash
export OPENAI_API_KEY=your_openai_api_key_here
export ANTHROPIC_API_KEY=your_anthropic_api_key_here
export GOOGLE_API_KEY=your_google_api_key_here
```

## Router Integration

### Model-Based Provider Selection

The router automatically determines the provider based on model naming conventions:

```rust
fn determine_provider_for_model(&self, model: &str) -> LLMProviderType {
    if model.starts_with("gpt-") {
        LLMProviderType::OpenAI
    } else if model.starts_with("gemini-") {
        LLMProviderType::Google
    } else if model.starts_with("claude-") {
        LLMProviderType::Anthropic
    } else {
        LLMProviderType::Anthropic // Default fallback
    }
}
```

### Cost Optimization

Each provider includes cost calculation functions:

```rust
// Example: Calculate costs for different providers
let openai_cost = calculate_openai_cost(input_tokens, output_tokens, "gpt-4");
let google_cost = calculate_google_cost(input_tokens, output_tokens, "gemini-pro");
let anthropic_cost = calculate_anthropic_cost(input_tokens, output_tokens, "claude-3-sonnet");
```

## GraphQL Configuration

### Provider Configuration Mutation

```graphql
mutation ConfigureProvider($input: LlmproviderConfigInput!) {
  configureLlmProvider(input: $input) {
    id
    providerType
    name
    baseUrl
    models {
      id
      name
      costPerInputToken
      costPerOutputToken
      supportsStreaming
      supportsFunctionCalling
      capabilities
    }
    healthStatus {
      isHealthy
      lastCheck
    }
  }
}
```

### Example Configuration

```json
{
  "input": {
    "providerType": "openai",
    "name": "OpenAI",
    "baseUrl": "https://api.openai.com/v1",
    "apiKeyId": "openai-key-1",
    "models": [
      {
        "id": "gpt-4o",
        "name": "GPT-4o",
        "maxTokens": 16384,
        "contextWindow": 128000,
        "costPerInputToken": 0.000005,
        "costPerOutputToken": 0.000015,
        "supportsStreaming": true,
        "supportsFunctionCalling": true,
        "capabilities": ["text_generation", "vision", "multimodal"]
      }
    ]
  }
}
```

## Usage Examples

### Basic Chat Completion

```rust
use circuit_breaker::llm::{LLMRouter, LLMRequest, ChatMessage, MessageRole};

let router = LLMRouter::new().await?;
let request = LLMRequest {
    model: "gpt-4o".to_string(), // Automatically routes to OpenAI
    messages: vec![ChatMessage {
        role: MessageRole::User,
        content: "Explain quantum computing".to_string(),
        name: None,
        function_call: None,
    }],
    temperature: Some(0.7),
    max_tokens: Some(1000),
    // ... other parameters
};

let response = router.chat_completion(request).await?;
```

### Streaming Example

```rust
let stream = router.stream_chat_completion(request).await?;
while let Some(chunk_result) = stream.next().await {
    match chunk_result {
        Ok(chunk) => {
            for choice in &chunk.choices {
                print!("{}", choice.delta.content);
            }
        },
        Err(e) => eprintln!("Stream error: {}", e),
    }
}
```

## Cost Comparison

For a typical 100-token input / 50-token output request:

| Provider | Model | Cost | Notes |
|----------|-------|------|-------|
| Google | Gemini 1.5 Flash | $0.000022 | Most cost-effective |
| Google | Gemini Pro | $0.000125 | Good balance |
| Anthropic | Claude 3 Haiku | $0.000088 | Fast, economical |
| OpenAI | GPT-3.5 Turbo | $0.000200 | Popular choice |
| Google | Gemini 1.5 Pro | $0.000875 | Largest context |
| Anthropic | Claude 3.5 Sonnet | $0.001050 | High quality |
| OpenAI | GPT-4o | $0.001250 | Multimodal |
| OpenAI | GPT-4 | $0.006000 | Most expensive |

## Testing

The implementation includes comprehensive tests:

```bash
# Test provider creation
cargo test providers::tests::test_create_openai_provider
cargo test providers::tests::test_create_google_provider
cargo test providers::tests::test_create_anthropic_provider

# Test cost calculations
cargo test providers::tests::test_cost_calculations

# Run the multi-provider demo
cargo run --example multi_provider_demo
```

## Demo Script

The `multi_provider_demo.rs` example demonstrates:

1. **Provider Listing**: Shows all configured providers and their models
2. **Provider Configuration**: Configures OpenAI, Google, and Anthropic providers via GraphQL
3. **Cost Comparison**: Compares costs across all models for a sample request
4. **Live Streaming**: Tests real-time streaming with available API keys

## Future Enhancements

### Smart Routing
- Implement cost-based routing
- Add performance-based selection
- Support fallback chains

### Advanced Features
- Function calling support across providers
- Vision/multimodal routing
- Custom provider support

### Monitoring
- Real-time cost tracking
- Provider health monitoring
- Usage analytics

## Security Considerations

- API keys are handled securely through environment variables
- Provider isolation prevents cross-contamination
- Rate limiting and health checks protect against failures
- No API keys are logged or exposed in responses

This implementation provides a solid foundation for multi-provider LLM support while maintaining the existing Anthropic-focused architecture and extending it naturally to support additional providers.