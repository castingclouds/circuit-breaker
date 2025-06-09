# Circuit Breaker Unified Server Guide

This guide explains how to run and use the Circuit Breaker unified server, which provides both GraphQL and OpenAI-compatible REST API endpoints in a single process.

## Quick Start

```bash
# Clone and build
git clone <repository>
cd circuit-breaker
cargo build --release

# Set up environment (optional)
cp .env.example .env
# Edit .env with your API keys

# Run the server
cargo run --bin server
```

The server will start two endpoints:
- **GraphQL API**: http://localhost:4000 (with GraphiQL interface)
- **OpenAI API**: http://localhost:3000 (REST endpoints)

## Server Architecture

```text
Circuit Breaker Unified Server
├── GraphQL Server (Port 4000)
│   ├── GraphiQL Interface
│   ├── Workflow Management
│   ├── Agent Management
│   └── WebSocket Subscriptions
│
├── OpenAI API Server (Port 3000)
│   ├── Chat Completions
│   ├── Streaming Support
│   ├── Model Management
│   └── Cost Tracking
│
└── Shared Infrastructure
    ├── LLM Router
    ├── Cost Optimizer
    ├── Storage Backend
    └── Provider Management
```

## Configuration

### Environment Variables

#### Server Configuration
```bash
# GraphQL Server
GRAPHQL_PORT=4000           # Default: 4000
GRAPHQL_HOST=localhost      # Default: localhost

# OpenAI API Server
OPENAI_PORT=3000            # Default: 3000
OPENAI_HOST=0.0.0.0        # Default: 0.0.0.0
OPENAI_CORS_ENABLED=true    # Default: true
OPENAI_API_KEY_REQUIRED=false  # Default: false
OPENAI_ENABLE_STREAMING=true   # Default: true

# General
RUST_LOG=info              # Default: info
ENVIRONMENT=development    # Default: development
```

#### Storage Backend
```bash
# Storage Type
STORAGE_BACKEND=memory     # Options: memory, nats

# NATS Configuration (if using NATS)
NATS_URL=nats://localhost:4222  # Default NATS URL
```

#### LLM Provider API Keys
```bash
# Provider Keys (all optional)
OPENAI_API_KEY=sk-...      # OpenAI API key
ANTHROPIC_API_KEY=sk-...   # Anthropic API key
GOOGLE_API_KEY=...         # Google AI API key
OLLAMA_BASE_URL=http://localhost:11434  # Ollama server URL
```

### Configuration File (.env)

Create a `.env` file in the project root:

```env
# Server Configuration
GRAPHQL_PORT=4000
OPENAI_PORT=3000
RUST_LOG=info

# Storage
STORAGE_BACKEND=memory

# LLM Provider Keys
OPENAI_API_KEY=your-openai-key-here
ANTHROPIC_API_KEY=your-anthropic-key-here
```

## API Documentation

### GraphQL API (Port 4000)

The GraphQL API provides complete workflow and agent management capabilities.

#### Endpoints
- **GraphiQL Interface**: http://localhost:4000
- **GraphQL Endpoint**: http://localhost:4000/graphql
- **WebSocket**: ws://localhost:4000/ws

#### Example Queries

```graphql
# List all workflows
query {
  workflows {
    id
    name
    places
    transitions {
      id
      from_places
      to_place
    }
  }
}

# Create a workflow token
mutation {
  createToken(input: {
    workflow_id: "document_review"
    place: "draft"
    data: "{\"title\": \"My Document\"}"
  }) {
    id
    place
    data
  }
}

# Create an AI agent
mutation {
  createAgent(input: {
    name: "Content Reviewer"
    description: "Reviews document content"
    llm_provider: {
      provider_type: "openai"
      model: "gpt-4"
      api_key: "your-key"
    }
    system_prompt: "You are a helpful content reviewer."
  }) {
    id
    name
    status
  }
}
```

### OpenAI-Compatible REST API (Port 3000)

The REST API provides OpenAI-compatible endpoints for LLM interactions.

#### Endpoints

- **Chat Completions**: `POST /v1/chat/completions`
- **Models**: `GET /v1/models`
- **Health Check**: `GET /health`

#### Chat Completions

```bash
# Basic chat completion
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ]
  }'
```

Response:
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello! I'm doing well, thank you for asking. How can I assist you today?"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 9,
    "completion_tokens": 12,
    "total_tokens": 21
  }
}
```

#### Streaming Chat Completions

```bash
# Streaming chat completion
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Write a short poem"}
    ],
    "stream": true
  }'
```

Response (Server-Sent Events):
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Here"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":" is"},"finish_reason":null}]}

data: [DONE]
```

#### List Models

```bash
curl http://localhost:3000/v1/models
```

Response:
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677652288,
      "owned_by": "circuit-breaker",
      "provider": "openai",
      "context_window": 8192,
      "supports_streaming": true
    },
    {
      "id": "claude-3-opus",
      "object": "model", 
      "created": 1677652288,
      "owned_by": "circuit-breaker",
      "provider": "anthropic",
      "context_window": 200000,
      "supports_streaming": true
    }
  ]
}
```

## Advanced Features

### Provider Routing

Circuit Breaker automatically routes requests to the best available provider based on:

- **Cost Optimization**: Chooses the most cost-effective provider
- **Performance**: Routes to fastest responding providers
- **Load Balancing**: Distributes load across providers
- **Failover**: Automatically switches on provider failures

### Cost Tracking

The server includes built-in cost tracking and optimization:

```bash
# View cost analytics via GraphQL
curl -X POST http://localhost:4000/graphql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "{ costAnalytics { total_cost daily_usage provider_breakdown } }"
  }'
```

### Streaming Support

Both APIs support real-time streaming:

- **GraphQL**: WebSocket subscriptions for workflow updates
- **REST API**: Server-Sent Events for chat completions

## Storage Backends

### In-Memory Storage (Default)

```bash
STORAGE_BACKEND=memory
```

- ✅ No setup required
- ✅ Fast for development
- ❌ Data lost on restart
- ❌ Not suitable for production

### NATS Storage

```bash
STORAGE_BACKEND=nats
NATS_URL=nats://localhost:4222
```

- ✅ Persistent storage
- ✅ Distributed architecture
- ✅ Real-time updates
- ✅ Production ready

#### Setting up NATS

```bash
# Using Docker
docker run -p 4222:4222 nats:alpine --jetstream

# Or install locally
# macOS
brew install nats-server
nats-server --jetstream

# Linux
wget https://github.com/nats-io/nats-server/releases/download/v2.10.4/nats-server-v2.10.4-linux-amd64.zip
unzip nats-server-v2.10.4-linux-amd64.zip
./nats-server --jetstream
```

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server /usr/local/bin/server
EXPOSE 3000 4000
CMD ["server"]
```

```bash
# Build and run
docker build -t circuit-breaker .
docker run -p 3000:3000 -p 4000:4000 \
  -e OPENAI_API_KEY=your-key \
  circuit-breaker
```

### Docker Compose

```yaml
version: '3.8'
services:
  circuit-breaker:
    build: .
    ports:
      - "3000:3000"  # OpenAI API
      - "4000:4000"  # GraphQL API
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - STORAGE_BACKEND=nats
      - NATS_URL=nats://nats:4222
    depends_on:
      - nats

  nats:
    image: nats:alpine
    command: ["--jetstream"]
    ports:
      - "4222:4222"
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: circuit-breaker
spec:
  replicas: 3
  selector:
    matchLabels:
      app: circuit-breaker
  template:
    metadata:
      labels:
        app: circuit-breaker
    spec:
      containers:
      - name: circuit-breaker
        image: circuit-breaker:latest
        ports:
        - containerPort: 3000
        - containerPort: 4000
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: llm-keys
              key: openai-key
---
apiVersion: v1
kind: Service
metadata:
  name: circuit-breaker-service
spec:
  selector:
    app: circuit-breaker
  ports:
  - name: openai-api
    port: 3000
    targetPort: 3000
  - name: graphql-api
    port: 4000
    targetPort: 4000
  type: LoadBalancer
```

## Monitoring and Logging

### Health Checks

```bash
# OpenAI API health
curl http://localhost:3000/health

# GraphQL API health
curl http://localhost:4000/health
```

### Logs

The server uses structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=debug

# Filter specific modules
export RUST_LOG=circuit_breaker=info,circuit_breaker::api=debug
```

### Metrics

Cost and usage metrics are available via GraphQL:

```graphql
query {
  costAnalytics {
    total_cost
    daily_usage
    monthly_usage
    provider_breakdown
    top_models
  }
}
```

## Security

### API Key Management

- **Optional Authentication**: Set `OPENAI_API_KEY_REQUIRED=true`
- **Provider Keys**: Securely stored and never logged
- **Rate Limiting**: Built-in rate limiting per API key

### Network Security

- **CORS**: Configurable CORS for browser access
- **TLS**: Use reverse proxy (nginx, Cloudflare) for HTTPS
- **Firewall**: Restrict access to management ports

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```
   Error: Address already in use (os error 48)
   ```
   Solution: Change ports or kill existing processes
   ```bash
   lsof -ti:3000 | xargs kill -9
   lsof -ti:4000 | xargs kill -9
   ```

2. **NATS Connection Failed**
   ```
   Error: Failed to connect to NATS server
   ```
   Solution: Ensure NATS server is running
   ```bash
   docker run -p 4222:4222 nats:alpine --jetstream
   ```

3. **Missing API Keys**
   ```
   Warning: No OpenAI API key found
   ```
   Solution: Set environment variables
   ```bash
   export OPENAI_API_KEY=your-key-here
   ```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
cargo run --bin server
```

### Testing

```bash
# Run all tests
cargo test

# Test specific features
cargo test --features nats
cargo test openai_api
```

## Examples

See the `examples/` directory for:
- Client implementations in multiple languages
- Integration examples
- Performance benchmarks
- Real-world use cases

## Support

- **Documentation**: [docs/](../docs/)
- **Examples**: [examples/](../examples/)
- **Issues**: GitHub Issues
- **Community**: Discord/Slack (links in README)