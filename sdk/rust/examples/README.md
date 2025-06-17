# Circuit Breaker Rust SDK Examples

This directory contains comprehensive examples demonstrating the Circuit Breaker Rust SDK capabilities.

## Prerequisites

1. **Circuit Breaker Server**: Ensure the Circuit Breaker server is running
   ```bash
   cargo run --bin server
   ```

2. **Environment Variables** (optional):
   ```bash
   export CIRCUIT_BREAKER_URL="http://localhost:4000"
   export CIRCUIT_BREAKER_API_KEY="your-api-key"  # if authentication is enabled
   ```

3. **Dependencies**: The examples use the SDK dependencies from Cargo.toml

## Examples Overview

### 1. Basic Usage (`basic_usage.rs`)

Demonstrates core SDK functionality including:

- ✅ **Client Creation**: Basic connection and authentication
- ✅ **Workflow Management**: Creating workflows with states and transitions
- ✅ **Resource Operations**: Creating and managing workflow resources
- ✅ **Rule Engine**: Server-side rule storage and evaluation with client-side fallback
- ✅ **Agent Creation**: AI agents with LLM integration
- ✅ **Multi-Provider LLM**: Testing OpenAI, Anthropic, and Google models
- ✅ **Smart Routing**: Virtual models and cost-optimized routing
- ✅ **Workflow Execution**: Running workflow instances
- ✅ **Resource Operations**: Managing resource states and data

**Run the example:**
```bash
cargo run --example basic_usage
```

### 2. Multi-Provider LLM Demo (`multi_provider_demo.rs`)

Comprehensive demonstration of Circuit Breaker's multi-provider LLM capabilities:

- 🏢 **Provider Discovery**: Automatic detection of configured LLM providers
- 🧪 **Individual Testing**: Provider-specific model validation and testing
- 💰 **Cost Analysis**: Real-time cost comparison across providers
- 🧠 **Smart Routing**: Virtual models and strategy-based provider selection
- 🎯 **Task-Specific Routing**: Automatic provider selection based on task type
- 🔧 **Advanced Features**: Temperature testing and SDK integration
- 📊 **Performance Metrics**: Latency and cost comparison

**Run the demo:**
```bash
cargo run --example multi_provider_demo
```

## Multi-Provider Configuration

The multi-provider demo showcases Circuit Breaker's ability to work with multiple LLM providers simultaneously:

### Supported Providers

| Provider | Models | Features |
|----------|--------|----------|
| **OpenAI** | GPT-4, GPT-4o-mini, GPT-3.5 | Function calling, streaming, embeddings |
| **Anthropic** | Claude 3 (Haiku, Sonnet, Opus) | Long context, safety, reasoning |
| **Google** | Gemini 1.5 (Flash, Pro) | Multimodal, fast inference, large context |

### Virtual Models

Circuit Breaker provides virtual models that automatically route to the best provider:

- `auto` - Automatic routing based on request characteristics
- `cb:cost-optimal` - Routes to the most cost-effective provider
- `cb:fastest` - Routes to the fastest responding provider
- `cb:coding` - Optimized for code generation tasks
- `cb:creative` - Optimized for creative writing tasks

### Smart Routing Strategies

```rust
// Cost-optimized routing
let request = LLMRequest {
    model: "auto".to_string(),
    messages: vec![/* your messages */],
    // Circuit breaker routing would be configured server-side
    ..Default::default()
};

// Using chat builder with specific provider
let response = create_chat(COMMON_MODELS::CLAUDE_3_HAIKU)
    .set_system_prompt("You are a helpful assistant.")
    .add_user_message("Your question here")
    .set_temperature(0.7)
    .execute(&client.llm())
    .await?;
```

## Key Features Demonstrated

### 🔄 Workflow Engine
- State machine definition and execution
- Resource lifecycle management
- Builder pattern for easy workflow creation
- History tracking and auditing

### 🤖 AI Integration
- Multi-provider LLM support
- Agent creation and management
- Chat interfaces with builder patterns
- Cost optimization and routing

### 📋 Rule Engine
- Server-side rule storage
- Real-time rule evaluation
- Client-side fallback capabilities
- Flexible rule builder patterns

### 🔍 Monitoring & Analytics
- Provider health monitoring
- Cost tracking and analysis
- Performance metrics
- Error handling and fallbacks

## Running Examples

### Option 1: Cargo Run
```bash
# Run basic usage example
cargo run --example basic_usage

# Run multi-provider demo
cargo run --example multi_provider_demo

# Run other examples
cargo run --example workflow_management
cargo run --example llm_integration
```

### Option 2: Build and Run
```bash
# Build all examples
cargo build --examples

# Run specific example
./target/debug/examples/basic_usage
```

## Troubleshooting

### Common Issues

1. **Server Not Running**
   ```
   ❌ Failed to connect: Connection refused
   ```
   **Solution**: Start the Circuit Breaker server:
   ```bash
   cargo run --bin server
   ```

2. **API Key Required**
   ```
   ❌ Authentication failed
   ```
   **Solution**: Set your API key:
   ```bash
   export CIRCUIT_BREAKER_API_KEY="your-api-key"
   ```

3. **LLM Provider Unavailable**
   ```
   ⚠️ Agent creation skipped (LLM not available)
   ```
   **Solution**: Configure LLM provider API keys in the server's `.env` file

4. **Port Conflicts**
   ```
   ❌ Server not responding
   ```
   **Solution**: Check if ports 3000 (OpenAI API) and 4000 (GraphQL) are available

### Debug Mode

Enable verbose logging by setting the Rust log level:

```bash
RUST_LOG=debug cargo run --example basic_usage
```

## SDK Features Showcased

### Builder Patterns
```rust
// Workflow builder
let workflow = create_workflow("My Workflow")
    .set_description("A sample workflow")
    .add_state("pending", "normal")
    .add_state("completed", "final")
    .add_transition("pending", "completed", "complete")
    .set_initial_state("pending")
    .build();

// Agent builder
let agent = create_agent("Support Agent")
    .set_description("Customer support AI")
    .set_type("conversational")
    .set_llm_provider("openai")
    .set_model(COMMON_MODELS::GPT_4O_MINI)
    .set_temperature(0.7)
    .build();

// Chat builder
let response = create_chat(COMMON_MODELS::CLAUDE_3_HAIKU)
    .set_system_prompt("You are a helpful assistant.")
    .add_user_message("Hello!")
    .set_temperature(0.3)
    .execute(&client.llm())
    .await?;
```

### Async/Await Support
All SDK operations are async and work with Tokio:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .base_url("http://localhost:4000")?
        .build()?;
    
    let response = client.llm()
        .chat("gpt-4o-mini", "Hello!")
        .await?;
    
    println!("Response: {}", response);
    Ok(())
}
```

## Integration Guide

To integrate these examples into your own application:

1. **Add the SDK dependency**:
   ```toml
   [dependencies]
   circuit-breaker-sdk = "0.1.0"
   tokio = { version = "1.0", features = ["full"] }
   ```

2. **Import the SDK**:
   ```rust
   use circuit_breaker_sdk::{
       Client, create_workflow, create_agent, create_chat,
       COMMON_MODELS, Result
   };
   ```

3. **Initialize the client**:
   ```rust
   let client = Client::builder()
       .base_url("http://localhost:4000")?
       .api_key(std::env::var("CIRCUIT_BREAKER_API_KEY").ok())
       .build()?;
   ```

4. **Use the patterns from the examples** to build your workflow automation and AI integration.

## Performance Considerations

- All operations are async and non-blocking
- The SDK uses connection pooling for HTTP requests
- GraphQL queries are optimized for minimal data transfer
- Error handling is built-in with detailed error types

## Additional Examples

More examples coming soon:

- `workflow_management.rs` - Advanced workflow patterns
- `function_chains.rs` - Function composition and chaining
- `ai_agent.rs` - Advanced AI agent configurations
- `rules_demo.rs` - Complex rule engine scenarios
- `llm_integration.rs` - Deep LLM integration patterns

## Contributing

Found an issue or want to add more examples? Please:

1. Fork the repository
2. Create a feature branch
3. Add your example with proper documentation
4. Submit a pull request

---

**Happy automating with Circuit Breaker! 🦀🚀**