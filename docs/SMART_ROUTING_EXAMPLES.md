# Circuit Breaker Smart Routing Examples

This document provides comprehensive examples of how to use Circuit Breaker's smart routing capabilities while maintaining 100% OpenAI API compatibility.

## Table of Contents

1. [Quick Start](#quick-start)
2. [OpenAI API Compatibility](#openai-api-compatibility)
3. [Virtual Model Names](#virtual-model-names)
4. [Smart Routing with Preferences](#smart-routing-with-preferences)
5. [Task-Specific Routing](#task-specific-routing)
6. [Streaming with Smart Routing](#streaming-with-smart-routing)
7. [Cost Optimization](#cost-optimization)
8. [Language Examples](#language-examples)

## Quick Start

```bash
# Start the Circuit Breaker server
cargo run --bin server

# Server provides two APIs:
# - OpenAI API: http://localhost:3000
# - GraphQL API: http://localhost:4000
```

## OpenAI API Compatibility

Circuit Breaker is a **drop-in replacement** for OpenAI API. All existing OpenAI code works unchanged:

### Basic Example
```bash
# Works exactly like OpenAI API
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Using OpenAI SDK
```python
# Python - just change the base URL
import openai

client = openai.OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="not-needed"  # Optional with Circuit Breaker
)

response = client.chat.completions.create(
    model="claude-3-haiku-20240307",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

```javascript
// JavaScript - just change the base URL
import OpenAI from 'openai';

const openai = new OpenAI({
  baseURL: 'http://localhost:3000/v1',
  apiKey: 'not-needed'
});

const response = await openai.chat.completions.create({
  model: 'claude-3-haiku-20240307',
  messages: [{ role: 'user', content: 'Hello!' }]
});
```

## Virtual Model Names

Use virtual models for automatic provider selection:

### Available Virtual Models

| Virtual Model | Description | Strategy |
|---------------|-------------|----------|
| `auto` | Let Circuit Breaker choose the best model | Balanced |
| `cb:smart-chat` | Smart chat model selection | Balanced |
| `cb:cost-optimal` | Most cost-effective model | Cost Optimized |
| `cb:fastest` | Fastest responding model | Performance First |
| `cb:coding` | Best for code generation | Task Specific |
| `cb:analysis` | Best for data analysis | Task Specific |
| `cb:creative` | Best for creative writing | Task Specific |

### Examples

```bash
# Auto-select best model
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Cost-optimized selection
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:cost-optimal",
    "messages": [{"role": "user", "content": "Explain quantum computing"}]
  }'

# Best model for coding
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:coding",
    "messages": [{"role": "user", "content": "Write a Python web scraper"}]
  }'
```

## Smart Routing with Preferences

Add `circuit_breaker` configuration to any OpenAI request for smart routing:

### Basic Smart Routing
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "Hello!"}],
    "circuit_breaker": {
      "routing_strategy": "cost_optimized"
    }
  }'
```

### Available Strategies

- `cost_optimized` - Choose cheapest available provider
- `performance_first` - Choose fastest responding provider  
- `balanced` - Balance cost and performance
- `reliability_first` - Choose most reliable provider
- `task_specific` - Choose based on task type

### With Constraints
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Write a business plan"}],
    "circuit_breaker": {
      "routing_strategy": "cost_optimized",
      "max_cost_per_1k_tokens": 0.002,
      "max_latency_ms": 3000,
      "fallback_models": ["claude-3-haiku-20240307", "gpt-3.5-turbo"]
    }
  }'
```

## Task-Specific Routing

Optimize model selection for specific tasks:

### Code Generation
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:coding",
    "messages": [{"role": "user", "content": "Create a REST API in Python with FastAPI"}]
  }'
```

### Data Analysis
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:analysis",
    "messages": [{"role": "user", "content": "Analyze this sales data: [1000, 1200, 900, 1400, 1100]"}]
  }'
```

### Creative Writing
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:creative",
    "messages": [{"role": "user", "content": "Write a short story about time travel"}]
  }'
```

## Streaming with Smart Routing

All smart routing features work with streaming:

```bash
# Smart streaming
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:smart-chat",
    "messages": [{"role": "user", "content": "Write a poem about AI"}],
    "stream": true,
    "circuit_breaker": {
      "routing_strategy": "performance_first"
    }
  }'
```

## Cost Optimization

Control costs while maintaining quality:

### Set Budget Limits
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Explain machine learning"}],
    "circuit_breaker": {
      "routing_strategy": "cost_optimized",
      "max_cost_per_1k_tokens": 0.001
    }
  }'
```

### Performance vs Cost Trade-off
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Quick question: What is 2+2?"}],
    "circuit_breaker": {
      "routing_strategy": "balanced",
      "max_cost_per_1k_tokens": 0.005,
      "max_latency_ms": 2000
    }
  }'
```

## Language Examples

### Python with Smart Routing

```python
import openai
import json

client = openai.OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="not-needed"
)

# Smart routing with preferences
def smart_completion(content, strategy="balanced", task_type=None, max_cost=None):
    circuit_breaker_config = {
        "routing_strategy": strategy
    }
    
    if task_type:
        circuit_breaker_config["task_type"] = task_type
    if max_cost:
        circuit_breaker_config["max_cost_per_1k_tokens"] = max_cost
    
    response = client.chat.completions.create(
        model="auto",
        messages=[{"role": "user", "content": content}],
        extra_body={"circuit_breaker": circuit_breaker_config}
    )
    
    return response.choices[0].message.content

# Usage examples
print("Cost optimized:", smart_completion("Hello!", "cost_optimized"))
print("Coding task:", smart_completion("Write a function", "task_specific", "coding"))
print("Budget constrained:", smart_completion("Explain AI", "cost_optimized", max_cost=0.002))
```

### JavaScript/TypeScript

```typescript
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: 'http://localhost:3000/v1',
  apiKey: 'not-needed'
});

interface SmartRoutingConfig {
  routing_strategy?: string;
  task_type?: string;
  max_cost_per_1k_tokens?: number;
  max_latency_ms?: number;
}

async function smartCompletion(
  content: string, 
  config: SmartRoutingConfig = {}
) {
  const response = await client.chat.completions.create({
    model: 'auto',
    messages: [{ role: 'user', content }],
    circuit_breaker: config
  } as any);
  
  return response.choices[0].message.content;
}

// Usage examples
console.log(await smartCompletion("Hello!", { routing_strategy: "cost_optimized" }));
console.log(await smartCompletion("Write code", { 
  routing_strategy: "task_specific", 
  task_type: "coding" 
}));
```

### Rust

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    
    // Smart routing request
    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .json(&json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "Explain Rust ownership"}],
            "circuit_breaker": {
                "routing_strategy": "task_specific",
                "task_type": "coding",
                "max_cost_per_1k_tokens": 0.01
            }
        }))
        .send()
        .await?;
    
    let result: serde_json::Value = response.json().await?;
    println!("Response: {}", result["choices"][0]["message"]["content"]);
    
    Ok(())
}
```

### Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
)

type ChatRequest struct {
    Model         string                 `json:"model"`
    Messages      []Message              `json:"messages"`
    CircuitBreaker map[string]interface{} `json:"circuit_breaker,omitempty"`
}

type Message struct {
    Role    string `json:"role"`
    Content string `json:"content"`
}

func main() {
    request := ChatRequest{
        Model: "auto",
        Messages: []Message{
            {Role: "user", Content: "Write a Go function"},
        },
        CircuitBreaker: map[string]interface{}{
            "routing_strategy": "task_specific",
            "task_type":        "coding",
        },
    }
    
    jsonData, _ := json.Marshal(request)
    resp, err := http.Post(
        "http://localhost:3000/v1/chat/completions",
        "application/json",
        bytes.NewBuffer(jsonData),
    )
    
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()
    
    var result map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&result)
    
    fmt.Println("Response:", result["choices"].([]interface{})[0].(map[string]interface{})["message"].(map[string]interface{})["content"])
}
```

## Migration Guide

### From OpenAI API

```diff
# No changes needed - just change the URL
- openai.api_base = "https://api.openai.com/v1"
+ openai.api_base = "http://localhost:3000/v1"
```

### From OpenRouter

```diff
# Change URL and optionally add smart routing
- const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
+ const response = await fetch('http://localhost:3000/v1/chat/completions', {
    method: 'POST',
    headers: {
-     'Authorization': 'Bearer YOUR_OPENROUTER_KEY',
-     'HTTP-Referer': 'https://yourapp.com',
-     'X-Title': 'Your App'
+     'Content-Type': 'application/json'
    },
    body: JSON.stringify({
-     model: 'openai/gpt-4',
+     model: 'auto',  // Smart routing
      messages: [{ role: 'user', content: 'Hello!' }],
+     circuit_breaker: {
+       routing_strategy: 'cost_optimized'
+     }
    })
  });
```

## Best Practices

1. **Start with OpenAI compatibility** - Existing code works unchanged
2. **Use virtual models** for new projects - `auto`, `cb:smart-chat`, etc.
3. **Add constraints gradually** - Start with basic routing, add cost/latency limits as needed
4. **Task-specific models** - Use `cb:coding`, `cb:analysis` for specialized tasks
5. **Monitor costs** - Use GraphQL API to track usage and optimize
6. **Fallback models** - Always specify fallback options for reliability

## Troubleshooting

### Common Issues

**Q: Virtual model not found**
```bash
# Make sure you're using the correct virtual model names
curl http://localhost:3000/v1/models  # List all available models
```

**Q: No providers available**
```bash
# Check that API keys are set
export ANTHROPIC_API_KEY=your_key_here
export OPENAI_API_KEY=your_key_here
```

**Q: Cost constraints too strict**
```json
{
  "circuit_breaker": {
    "max_cost_per_1k_tokens": 0.01,  // Increase this value
    "fallback_models": ["claude-3-haiku-20240307"]  // Add fallbacks
  }
}
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
cargo run --bin server
```

This will show detailed smart routing decisions in the logs.