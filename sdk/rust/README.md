# Circuit Breaker Rust SDK

A comprehensive Rust SDK for building and managing workflows using the Circuit Breaker workflow engine. This SDK provides type-safe APIs for workflows, resources, rules engine, functions, LLM integration, and AI agents.

[![Crates.io](https://img.shields.io/crates/v/circuit-breaker-sdk.svg)](https://crates.io/crates/circuit-breaker-sdk)
[![Documentation](https://docs.rs/circuit-breaker-sdk/badge.svg)](https://docs.rs/circuit-breaker-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **🔄 Workflow Management**: Build, validate, and execute complex workflows with state machines
- **📊 Resource Management**: Manage stateful resources with automatic state transitions
- **🧠 Rules Engine**: Define and evaluate business rules with multiple rule types (simple, composite, JavaScript)
- **🐳 Function System**: Execute containerized functions with Docker integration
- **🤖 LLM Router**: Route requests across multiple LLM providers with load balancing and failover
- **🎯 AI Agents**: Build conversational and state-machine based AI agents
- **⚡ Streaming Support**: Real-time streaming for LLM responses and workflow events
- **🔒 Type Safety**: Full Rust type safety with comprehensive error handling
- **📈 Performance**: Async/await throughout with connection pooling and caching

## Quick Start

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
circuit-breaker-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use circuit_breaker_sdk::{CircuitBreakerSDK, WorkflowBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SDK instance
    let sdk = CircuitBreakerSDK::new("http://localhost:4000/graphql").await?;

    // Build a workflow
    let workflow = WorkflowBuilder::new("Order Processing")
        .add_state("pending")
        .add_state("processing") 
        .add_state("completed")
        .add_transition("pending", "processing", "start_processing")
        .add_transition("processing", "completed", "complete_order")
        .set_initial_state("pending")
        .build()?;

    // Create the workflow
    let workflow_id = sdk.workflows().create(workflow).await?;

    // Create a resource
    let resource = sdk.resources().create(&workflow_id, serde_json::json!({
        "orderId": "order-123",
        "amount": 99.99
    })).await?;

    println!("Created workflow: {} with resource: {}", workflow_id, resource.id);
    Ok(())
}
```

## Core Components

### Workflow Management

Build and manage complex workflows with state machines:

```rust
use circuit_breaker_sdk::{WorkflowBuilder, create_workflow};

// Simple linear workflow
let workflow = create_linear_workflow(
    "Order Process",
    vec!["received", "validated", "processed", "shipped", "delivered"]
).build()?;

// Complex workflow with branching
let workflow = WorkflowBuilder::new("Payment Processing")
    .add_state("pending")
    .add_state("authorized") 
    .add_state("captured")
    .add_state("failed")
    .add_transition("pending", "authorized", "authorize_payment")
    .add_transition("authorized", "captured", "capture_payment")
    .add_transition("pending", "failed", "authorization_failed")
    .build()?;
```

### Resource Management

Manage stateful resources that flow through workflows:

```rust
// Create a resource
let resource = sdk.resources().create(&workflow_id, serde_json::json!({
    "customerId": "cust_123",
    "amount": 250.00,
    "currency": "USD"
})).await?;

// Execute an activity to transition state
use circuit_breaker_sdk::ActivityExecuteInput;

let result = sdk.resources().execute_activity(ActivityExecuteInput {
    resource_id: resource.id,
    activity_name: "authorize_payment".to_string(),
    context: Some(serde_json::json!({"provider": "stripe"}).as_object().unwrap().clone()),
    force: false,
}).await?;
```

### Rules Engine

Define and evaluate business rules:

```rust
use circuit_breaker_sdk::{RuleBuilder, create_rule};

// Simple field-based rule
let rule = create_rule("amount_check")
    .field_greater_than("amount", 100.0)
    .description("Amount must be greater than $100")
    .build()?;

// Complex composite rule
let rule = RuleBuilder::new("payment_validation")
    .and(vec![
        create_rule("amount_positive").field_greater_than("amount", 0.0).build()?,
        create_rule("currency_valid").field_contains("currency", "USD,EUR,GBP").build()?,
    ])
    .description("Validate payment details")
    .build()?;

// Register and evaluate
let rule_id = sdk.rules().register_rule(rule).await?;
let result = sdk.rules().evaluate(context).await?;
```

### Function System (Docker Integration)

Execute containerized functions as part of workflows:

```rust
use circuit_breaker_sdk::{FunctionBuilder, ContainerConfig};

let function = FunctionBuilder::new("payment_processor")
    .description("Process payment using external service")
    .container(ContainerConfig {
        image: "my-org/payment-processor:v1.0".to_string(),
        command: Some(vec!["python".to_string(), "process.py".to_string()]),
        environment: [
            ("API_KEY".to_string(), "${PAYMENT_API_KEY}".to_string()),
            ("ENVIRONMENT".to_string(), "production".to_string()),
        ].into(),
        ..Default::default()
    })
    .input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "amount": {"type": "number"},
            "currency": {"type": "string"},
            "customer_id": {"type": "string"}
        },
        "required": ["amount", "currency", "customer_id"]
    }))
    .build()?;

let function_id = sdk.functions().create(function).await?;

// Execute the function
let result = sdk.functions().execute(function_id, serde_json::json!({
    "amount": 99.99,
    "currency": "USD", 
    "customer_id": "cust_123"
})).await?;
```

### LLM Integration

Route requests across multiple LLM providers:

```rust
use circuit_breaker_sdk::{ChatCompletionRequest, ChatMessage, ChatRole};

// Simple chat completion
let request = ChatCompletionRequest {
    model: "gpt-4".to_string(),
    messages: vec![
        ChatMessage {
            role: ChatRole::System,
            content: Some("You are a helpful assistant.".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: Some("What is the meaning of life?".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ],
    temperature: Some(0.7),
    max_tokens: Some(150),
    stream: Some(false),
    tools: None,
    tool_choice: None,
    additional_params: None,
};

let response = sdk.llm().chat_completion(request).await?;
println!("Response: {}", response.choices[0].message.content.as_ref().unwrap());
```

### AI Agents

Build intelligent agents with conversation and workflow capabilities:

```rust
// Conversational agent
let agent = sdk.agent_builder("Customer Support")
    .conversational()
    .system_prompt("You are a helpful customer support agent.")
    .llm_provider("openai")
    .enable_memory()
    .build()
    .await?;

// State machine agent
let agent = sdk.agent_builder("Order Assistant")
    .state_machine()
    .add_state("greeting", "Welcome! How can I help with your order?")
    .add_state("collecting_info", "Please provide your order number.")
    .add_state("processing", "Let me look up your order...")
    .add_state("complete", "Thank you! Is there anything else?")
    .add_transition("greeting", "collecting_info", "user_needs_help")
    .add_transition("collecting_info", "processing", "order_number_provided")
    .add_transition("processing", "complete", "order_found")
    .set_initial_state("greeting")
    .build()
    .await?;
```

## Configuration

### Environment Variables

```bash
# Core configuration
export CIRCUIT_BREAKER_GRAPHQL_ENDPOINT="http://localhost:4000/graphql"
export CIRCUIT_BREAKER_TIMEOUT_MS=30000
export CIRCUIT_BREAKER_RETRY_ATTEMPTS=3
export CIRCUIT_BREAKER_DEBUG=true

# Authentication
export CIRCUIT_BREAKER_API_KEY="your-api-key"
# or
export CIRCUIT_BREAKER_TOKEN="your-bearer-token"
```

### Configuration File

Create a `config.yaml` file:

```yaml
graphql_endpoint: "http://localhost:4000/graphql"
timeout_ms: 30000
retry_attempts: 3
debug: false

auth:
  type: "api_key"
  api_key: "${API_KEY}"

llm:
  default_provider: "openai"
  providers:
    openai:
      provider_type: "openai"
      base_url: "https://api.openai.com/v1"
      api_key: "${OPENAI_API_KEY}"
      models: ["gpt-4", "gpt-3.5-turbo"]
      default_model: "gpt-4"
    
    anthropic:
      provider_type: "anthropic"
      base_url: "https://api.anthropic.com"
      api_key: "${ANTHROPIC_API_KEY}"
      models: ["claude-3-opus", "claude-3-sonnet"]

functions:
  docker:
    socket_url: "unix:///var/run/docker.sock"
  default_timeout_ms: 300000
  max_concurrent: 10

rules:
  enable_cache: true
  cache_size: 1000
  allow_javascript: false

logging:
  level: "info"
  format: "pretty"
  enable_console: true
```

Load configuration:

```rust
use circuit_breaker_sdk::{SDKConfig, CircuitBreakerSDK};

// From environment
let config = SDKConfig::from_env()?;
let sdk = CircuitBreakerSDK::with_config(config).await?;

// From file
let config = SDKConfig::from_file("config.yaml")?;
let sdk = CircuitBreakerSDK::with_config(config).await?;

// Programmatic configuration
let config = SDKConfig::default()
    .with_endpoint("http://localhost:4000/graphql")
    .with_timeout(60000)
    .with_debug(true)
    .with_header("x-api-key", "your-key");

let sdk = CircuitBreakerSDK::with_config(config).await?;
```

## Feature Flags

Enable specific features in your `Cargo.toml`:

```toml
[dependencies]
circuit-breaker-sdk = { version = "0.1.0", features = [
    "docker",        # Docker/container integration
    "streaming",     # Streaming support for LLM responses
    "webhooks",      # Webhook server for events
    "llm-all",       # All LLM providers
    "metrics",       # Metrics collection
] }
```

Available features:
- `docker` - Docker integration for function execution
- `streaming` - Streaming support for real-time responses
- `webhooks` - HTTP webhook server for event handling
- `llm-openai` - OpenAI provider support
- `llm-anthropic` - Anthropic provider support
- `llm-ollama` - Ollama provider support
- `llm-all` - All LLM providers
- `rules-javascript` - JavaScript rule evaluation
- `metrics` - Metrics and monitoring
- `validation-strict` - Enhanced validation

## Examples

Check out the examples in the `/examples` directory:

```bash
# Basic workflow creation
cargo run --example basic_workflow

# Workflow management
cargo run --example workflow_management

# Rules engine demo
cargo run --example rules_demo

# Function chains
cargo run --example function_chains --features docker

# LLM integration
cargo run --example llm_integration

# AI agent development
cargo run --example ai_agent
```

## Error Handling

The SDK provides comprehensive error handling with specific error types:

```rust
use circuit_breaker_sdk::{SDKError, WorkflowError, ResourceError};

match sdk.workflows().create(workflow).await {
    Ok(workflow_id) => println!("Created workflow: {}", workflow_id),
    Err(SDKError::Workflow(WorkflowError::InvalidDefinition { reason })) => {
        eprintln!("Invalid workflow: {}", reason);
    }
    Err(SDKError::Network(network_err)) => {
        eprintln!("Network error: {}", network_err);
    }
    Err(err) => {
        eprintln!("Unexpected error: {}", err);
    }
}
```

## Performance Tuning

### Connection Pooling

```rust
let config = SDKConfig::default()
    .with_performance(PerformanceConfig {
        connection_pool_size: 20,
        batch_size: 100,
        enable_caching: true,
        cache_size: 5000,
        cache_ttl_seconds: 600,
        ..Default::default()
    });
```

### Resource Limits

```rust
let function_config = FunctionConfigSection {
    default_limits: ResourceLimitsConfig {
        memory_bytes: Some(1024 * 1024 * 1024), // 1GB
        cpu_cores: Some(2.0),
        disk_bytes: Some(5 * 1024 * 1024 * 1024), // 5GB
        network_bps: None,
    },
    max_concurrent: 50,
    enable_caching: true,
    ..Default::default()
};
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests with all features
cargo test --all-features

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```bash
# Clone the repository
git clone https://github.com/circuit-breaker/sdk.git
cd sdk/rust

# Install dependencies
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Documentation**: [https://docs.rs/circuit-breaker-sdk](https://docs.rs/circuit-breaker-sdk)
- **Issues**: [GitHub Issues](https://github.com/circuit-breaker/sdk/issues)
- **Discussions**: [GitHub Discussions](https://github.com/circuit-breaker/sdk/discussions)

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a list of changes and version history.