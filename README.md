# Circuit Breaker - Rust Edition

> A unified platform providing **State Managed Workflows** via GraphQL and **OpenAI-compatible LLM routing** via REST API  
> **Generic by design** - like Dagger's engine, the backend knows nothing about your domain  
> **OpenRouter Alternative** - BYOK (Bring Your Own Key) model with intelligent provider routing  
> **Local AI Support** - Full Ollama integration with automatic model detection and streaming

## 🚀 Project Vision

Circuit Breaker is a **distributed, high-performance platform** that combines workflow orchestration with intelligent LLM routing. It provides two complementary APIs:

1. **State Managed Workflows** - Powered by Petri Nets for mathematical rigor and formal workflow verification
2. **LLM Provider Routing** - OpenAI-compatible API with cost optimization and intelligent failover
3. **Local AI Integration** - Native Ollama support with automatic model discovery and async loading
4. **Multi-Provider Support** - OpenAI, Anthropic, Google, Azure OpenAI, Ollama, and custom endpoints

**Key Principles**: 
- **Unified Server**: Single binary providing both GraphQL and REST APIs
- **OpenAI Compatible**: Drop-in replacement for OpenRouter.ai with BYOK model
- **Local AI First**: Native Ollama integration with zero-config model detection
- **State Managed Workflows**: Unlike DAG-based systems, supports cycles, concurrent states, and complex relationships
- **Mathematical Guarantees**: Petri Net formalism provides deadlock detection and state safety
- **Polyglot First**: Any language can use either GraphQL or REST APIs

## 🚀 Quick Start

### 1. Start the Server

```bash
# Clone and build
git clone <repository>
cd circuit-breaker
cargo build --release

# Optional: Add your API keys for smart routing
cp .env.example .env
# Edit .env with your OpenAI, Anthropic, etc. keys

# For local AI with Ollama (requires Ollama running)
# Install Ollama: https://ollama.ai
ollama pull qwen2.5-coder:3b
ollama pull gemma2:2b
ollama pull nomic-embed-text:latest

# Start unified server (both GraphQL + OpenAI API)
cargo run --bin server
```

The server starts two APIs:
- **GraphQL API**: http://localhost:4000 (Workflow management)
- **OpenAI API**: http://localhost:3000 (LLM routing with smart features)

### 2. Try OpenAI-Compatible API (100% Compatible)

```bash
# Works exactly like OpenAI API - with remote providers
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Or with local Ollama models
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen2.5-coder:3b",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# List all models (real + virtual)
curl http://localhost:3000/v1/models

# Try embeddings with local models
curl -X POST http://localhost:3000/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "nomic-embed-text:latest",
    "input": "Hello, world!"
  }'
```

## 🤖 Ollama Integration

Circuit Breaker provides **first-class Ollama support** with automatic model detection, async loading, and full OpenAI API compatibility.

### Supported Models

**Coding & Development**
- `qwen2.5-coder:3b` - Lightweight coding assistant (recommended)
- `qwen2.5-coder:7b` - Advanced coding with better context
- `codellama:7b` - Meta's Code Llama for code generation

**Text Generation**
- `gemma2:2b` - Fast, efficient text generation
- `llama3.1:8b` - High-quality general purpose model
- `mistral:7b` - Balanced performance and quality

**Embeddings**
- `nomic-embed-text:latest` - Text embeddings for semantic search
- `all-minilm:l6-v2` - Lightweight sentence embeddings

### Features

✅ **Automatic Model Detection** - Discovers available models on startup  
✅ **Async Model Loading** - Non-blocking model initialization  
✅ **Streaming Chat Completions** - Real-time response streaming  
✅ **Embeddings Support** - Vector embeddings for semantic operations  
✅ **OpenAI API Compatibility** - Drop-in replacement for OpenAI clients  
✅ **Dynamic Model Management** - Hot-reload models without restart  
✅ **Performance Optimized** - Rust async for maximum throughput  

### Quick Ollama Setup

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull recommended models
ollama pull qwen2.5-coder:3b      # 2GB - Coding
ollama pull gemma2:2b             # 1.6GB - Text 
ollama pull nomic-embed-text      # 274MB - Embeddings

# Start Circuit Breaker (auto-detects models)
cargo run --bin server

# Test local AI
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "qwen2.5-coder:3b",
    "messages": [{"role": "user", "content": "Write a Rust function to calculate fibonacci"}],
    "stream": true
  }'
```

## 🌐 Supported AI Providers

Circuit Breaker supports a comprehensive range of AI providers with unified OpenAI-compatible APIs:

### Cloud Providers

**OpenAI**
- All GPT models (gpt-4, gpt-3.5-turbo, etc.)
- Text embeddings (text-embedding-ada-002, text-embedding-3-small/large)
- Vision and multimodal support

**Anthropic**
- Claude 3 family (claude-3-opus, claude-3-sonnet, claude-3-haiku)
- Claude 2.1 and 2.0
- Large context windows (up to 200k tokens)

**Google**
- Gemini Pro and Gemini Pro Vision
- PaLM 2 models
- Vertex AI integration

**Azure OpenAI**
- All OpenAI models via Azure
- Custom deployment names
- Regional availability

### Local AI

**Ollama** (First-class support)
- Automatic model detection
- Async model loading
- Streaming responses
- Embeddings support
- 50+ models available

**Custom Endpoints**
- Any OpenAI-compatible API
- Self-hosted models
- Custom authentication

### Configuration Examples

```bash
# Environment variables
OPENAI_API_KEY=sk-your-key
ANTHROPIC_API_KEY=sk-ant-your-key
GOOGLE_API_KEY=your-google-key
AZURE_OPENAI_API_KEY=your-azure-key
AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com
OLLAMA_BASE_URL=http://localhost:11434

# Multiple keys for load balancing
OPENAI_API_KEYS=sk-key1,sk-key2,sk-key3

# Custom endpoints
CUSTOM_LLM_ENDPOINT=https://your-api.com/v1
CUSTOM_LLM_API_KEY=your-key
```

### 3. Try Smart Routing Features

```bash
# Auto-select best model
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "auto",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Cost-optimized routing
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "cb:cost-optimal",
    "messages": [{"role": "user", "content": "Explain quantum computing"}]
  }'

# Smart routing with preferences
curl -X POST http://localhost:3000/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Write code"}],
    "circuit_breaker": {
      "routing_strategy": "cost_optimized",
      "max_cost_per_1k_tokens": 0.002,
      "task_type": "coding"
    }
  }'
```

### 3. Try the GraphQL API

Visit http://localhost:4000 for the GraphiQL interface, or:

```bash
# Create a workflow
curl -X POST http://localhost:4000/graphql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "mutation { createWorkflow(input: {name: \"Test\", places: [\"start\", \"end\"], transitions: [{id: \"go\", from_places: [\"start\"], to_place: \"end\"}]}) { id name } }"
  }'

# Create and manage AI agents
curl -X POST http://localhost:4000/graphql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "mutation { createAgent(input: {name: \"Helper\", description: \"AI Assistant\", llm_provider: {provider_type: \"openai\", model: \"gpt-4\", api_key: \"your-key\"}}) { id status } }"
  }'
```

## 🏗️ Architecture Overview

### Clean Layer Separation

```
src/
├── models/           # 📦 Core domain logic (language-agnostic)
│   ├── token.rs      # Generic token and history tracking
│   └── workflow.rs   # Generic state and transition definitions
├── engine/           # 🚀 Execution engines and APIs  
│   ├── graphql.rs    # GraphQL API implementation
│   └── storage.rs    # Storage abstraction (NATS, PostgreSQL, etc.)
├── server/           # 🖥️  Deployable server implementations
│   ├── graphql.rs    # Production GraphQL server with CORS, logging
│   └── playground.html # Interactive GraphQL interface
├── bin/              # 📡 Executable binaries
│   └── server.rs     # Main Circuit Breaker server
└── lib.rs            # Clean exports and error types

examples/             # 📚 Client examples only (no servers!)
├── rust/             # Rust client examples
│   ├── basic_workflow.rs # Direct model usage (server-side style)
│   ├── token_demo.rs     # Core token operations
│   └── graphql_client.rs # Rust as GraphQL client (distributed style)
└── typescript/       # TypeScript client examples
    ├── basic_workflow.ts # GraphQL client demo
    ├── token_demo.ts     # Token operations via GraphQL
    └── README.md         # TypeScript setup instructions
```

### Unified Server Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                🦀 Circuit Breaker Unified Server            │
│                  cargo run --bin server                     │  
├─────────────────────────────────────────────────────────────┤
│  📊 GraphQL API (Port 4000)    🤖 OpenAI API (Port 3000)   │
│  • Workflow Management         • Chat Completions          │
│  • Agent Orchestration         • Streaming Support         │
│  • Real-time Updates           • Model Management          │
│  • GraphiQL Interface          • Cost Optimization         │
└─────────────────────┬───────────────────┬───────────────────┘
                      │                   │
        ┌─────────────┼─────────────┐     │
        │             │             │     │
   🦀 Rust       📜 TypeScript   🐍 Python  │
   Clients        Clients       Clients    │
     │              │             │        │
 Direct Models   GraphQL Only  GraphQL Only │
 GraphQL Client                             │
                                           │
        ┌──────────────────────────────────┘
        │
   🔗 Any OpenAI-Compatible Client
   • curl, HTTPie, Postman
   • OpenAI Python/JS SDKs  
   • LangChain, AutoGPT
   • Custom Applications
```

## 🎯 State Managed Workflows vs. DAG Systems

### Why State Management Beats DAGs

| Feature | **State Managed Workflows** | **DAG-Based Systems** |
|---------|------------------------------|------------------------|
| **Cycles** | ✅ Revision loops, retries | ❌ Acyclic by definition |
| **Concurrent States** | ✅ Multiple tokens parallel | ❌ Single execution path |
| **Rollbacks** | ✅ Natural state reversions | ❌ Requires restart |
| **Complex Joins** | ✅ Petri Net synchronization | ⚠️ Limited patterns |
| **State Persistence** | ✅ Durable state management | ⚠️ Task-based only |
| **Mathematical Verification** | ✅ Formal analysis | ❌ No guarantees |

### Real-World Example: Document Approval

```graphql
# GraphQL Schema - Client defines domain logic
mutation CreateDocumentWorkflow {
  createWorkflow(input: {
    name: "Document Approval"
    states: ["draft", "review", "approved", "rejected"]
    transitions: [
      { id: "submit", fromStates: ["draft"], toState: "review" }
      { id: "approve", fromStates: ["review"], toState: "approved" }
      { id: "reject", fromStates: ["review"], toState: "rejected" }
      { id: "revise", fromStates: ["rejected"], toState: "draft" }  # Cycle!
    ]
    initialState: "draft"
  }) {
    id
    name
  }
}
```

```rust
// Rust Backend - Completely generic, no domain knowledge
let mut token = Token::new("document_workflow", StateId::from("draft"));
token.transition_to(StateId::from("review"), TransitionId::from("submit"));
```

## 🛠️ Technology Stack

### Core Engine: **Rust** 🦀
- **Generic Design**: Zero hardcoded domain knowledge
- **Performance**: ~10x faster than equivalent Python/Ruby
- **Memory Safety**: Zero-cost abstractions without garbage collection
- **Concurrency**: Native async/await with Tokio
- **Type Safety**: Compile-time workflow validation

### AI Integration
- **Multi-Provider Support**: OpenAI, Anthropic, Google, Azure OpenAI, Ollama
- **Local AI**: First-class Ollama integration with auto-detection
- **OpenAI Compatibility**: Drop-in replacement for existing applications
- **Streaming**: Real-time chat completions and embeddings
- **BYOK Model**: Bring-your-own-key with cost optimization
- **MCP Server**: Secure agent coordination with GitHub Apps-style auth

### Infrastructure
- **Message Bus/Eventing**: NATS JetStream for distributed workflows and token persistence
- **API**: Dual APIs - GraphQL (async-graphql) for workflows, REST for LLM routing
- **Web**: Axum for high-performance HTTP with WebSocket support
- **Storage**: Pluggable backends (NATS KV, PostgreSQL, etc.)
- **Streaming**: Multi-protocol support (SSE, WebSocket, GraphQL subscriptions)

**NATS Required**: The distributed workflow features require a NATS server with JetStream enabled. See [NATS Setup](#nats-server-setup-docker-with-rancher-desktop) below for quick Docker setup.

## ⚙️ Environment Configuration

### Option 1: Automated Setup (Recommended)

```bash
# Run the setup script for automatic configuration
./setup.sh
```

The setup script will:
- ✅ Check Rust and Node.js installations
- ✅ Copy `.env.example` to `.env`
- ✅ Build the project and install dependencies
- ✅ Run tests to verify setup
- ✅ Create helpful run scripts

### Option 2: Manual Setup

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Edit .env and add your primary API key:
ANTHROPIC_API_KEY=your_anthropic_api_key_here

# Optional: Add alternative providers (uncomment in .env if needed):
# OPENAI_API_KEY=your_openai_api_key_here
# GOOGLE_API_KEY=your_google_api_key_here

# 3. Build the project
cargo build
```

### Get API Keys

- **Anthropic** (Primary): https://console.anthropic.com/
- **OpenAI** (Alternative): https://platform.openai.com/api-keys
- **Google Gemini** (Alternative): https://makersuite.google.com/app/apikey
- **Ollama** (Local): Self-hosted (no API key needed)

**Note**: Only Anthropic API key is required by default. Other providers are available as alternatives. API keys are only needed for AI Agent features - basic workflow functionality works without them.

### NATS Server Setup (Docker with Rancher Desktop)

For the NATS JetStream integration, you'll need a NATS server running. The easiest way is using Docker with Rancher Desktop:

#### Option 1: Quick Start (Single Container)
```bash
# Start NATS with JetStream enabled
docker run -d \
  --name nats-jetstream \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:latest \
  -js \
  -m 8222

# Verify NATS is running
docker logs nats-jetstream
```

#### Option 2: Production Setup (Docker Compose)
Create a `docker-compose.nats.yml` file:

```yaml
version: '3.8'
services:
  nats:
    image: nats:latest
    container_name: nats-jetstream
    ports:
      - "4222:4222"    # NATS client port
      - "8222:8222"    # NATS monitoring port
      - "6222:6222"    # NATS cluster port
    command: ["-js", "-m", "8222", "-D"]
    volumes:
      - nats-storage:/data
    restart: unless-stopped

volumes:
  nats-storage:
```

```bash
# Start NATS with persistence
docker-compose -f docker-compose.nats.yml up -d

# Check NATS status
curl http://localhost:8222/varz
```

#### Environment Configuration
Add to your `.env` file:

```bash
# NATS Configuration
NATS_URL=nats://localhost:4222
NATS_CLUSTER_NAME=circuit-breaker-cluster
NATS_ENABLE_JETSTREAM=true
```

#### Verify Setup
```bash
# Install NATS CLI (optional but helpful)
# macOS with Homebrew:
brew install nats-io/nats-tools/nats

# Test connection
nats --server=localhost:4222 server info

# List JetStream streams (should be empty initially)
nats --server=localhost:4222 stream list
```

**Rancher Desktop Notes:**
- Ensure Rancher Desktop is running with Docker (containerd) enabled
- Ports 4222 and 8222 will be accessible from your host machine
- Data persists in Docker volumes between container restarts
- Use `docker ps` to verify the container is running

## 🚀 Quick Start

### 1. Start the Main Server

```bash
cargo run --bin server
```

**Server starts with:**
- 🌐 GraphQL API at `http://localhost:4000/graphql`
- 📊 Interactive Playground for testing
- ✅ Pre-loaded example workflows (Document Approval, Deployment Pipeline)

### 2. Try Client Examples

**Rust Clients (multiple approaches):**
```bash
# Direct model usage (fastest, server-side style)
cargo run --example basic_workflow
cargo run --example token_demo

# GraphQL client (distributed systems, same API as other languages)
cargo run --example graphql_client

# LLM Router and AI infrastructure demo
cargo run --example llm_router_demo
```

**TypeScript Clients:**
```bash
cd examples/typescript
npm install

# TypeScript GraphQL clients
npm run start:basic
npm run start:demo
```

### 3. Architecture Demo

```bash
# Shows the separation of models, engine, and server layers
cargo run --example basic_workflow

# Demonstrates LLM routing with cost optimization
cargo run --example llm_router_demo
```

**Output:**
```
🔄 Circuit Breaker - Refactored Architecture Demo
==================================================
📁 src/models/     → Domain-agnostic workflow state management
🚀 src/engine/     → GraphQL API for polyglot clients  
🖥️  src/server/     → Deployable server implementations

✅ Created workflow using src/models/: Example Process
🎯 Created token using src/models/: uuid-here

🔄 Executing transitions using src/models/...
   ➡️  init -> processing
   ➡️  processing -> review  
   ➡️  review -> complete

🏗️  Complete Architecture Benefits:
   📦 src/models/  → Pure domain logic, zero external dependencies
   🚀 src/engine/  → GraphQL interface, swappable for gRPC, REST, etc.
   🖥️  src/server/  → Production-ready servers with config, logging, CORS
```

### 4. Interactive GraphQL Playground

Visit `http://localhost:4000/graphql` and run:

```graphql
# List available workflows
query {
  workflows {
    id
    name
    states
    transitions {
      id
      fromStates
      toState
    }
  }
}

# Create a token in a workflow
mutation {
  createToken(input: {
    workflowId: "document_approval_v1"
    data: {
      title: "My Document"
      author: "Circuit Breaker User"
    }
  }) {
    id
    state
    workflowId
  }
}

# Fire a transition
mutation {
  fireTransition(input: {
    tokenId: "YOUR_TOKEN_ID"
    transitionId: "submit"
  }) {
    id
    state
    history {
      transition
      fromState
      toState
      timestamp
    }
  }
}
```

## 🏛️ Architecture Benefits

### 1. **Single Server Binary**

```bash
# Production deployment - just one binary!
cargo run --bin server
# Serves ALL languages via GraphQL
```

### 2. **Complete Domain Flexibility**

The engine knows nothing about your domain - define any workflow:

```rust
// E-commerce order processing
let ecommerce = WorkflowDefinition {
    states: vec!["cart", "payment", "fulfillment", "shipped", "delivered"],
    // ...
};

// Software deployment pipeline  
let deployment = WorkflowDefinition {
    states: vec!["development", "staging", "production", "rollback"],
    // ...
};

// AI agent campaign coordination
let ai_campaign = WorkflowDefinition {
    states: vec!["planning", "research", "generation", "review", "published"],
    // ...
};
```

### 3. **True Polyglot Ecosystem**

Any language can define and execute workflows via the same GraphQL API:

```python
# Python client
import requests

response = requests.post("http://localhost:4000/graphql", json={
    "query": """
        mutation { createWorkflow(input: {
            name: "ML Pipeline"
            states: ["data_prep", "training", "evaluation", "deployment"]
            # ... same API as TypeScript, Rust, Go, Java
        }) { id } }
    """
})
```

```typescript
// TypeScript client - identical API
const client = new GraphQLClient("http://localhost:4000/graphql");
const workflow = await client.request(gql`
  mutation { createWorkflow(input: {
    name: "Content Workflow"
    states: ["draft", "editing", "published"]
    # ... same API as Python, Rust, Go, Java
  }) { id } }
`);
```

### 4. **Flexible Client Patterns**

**Option A: Direct Rust Models (fastest)**
```rust
// examples/rust/basic_workflow.rs
use circuit_breaker::{Token, StateId, WorkflowDefinition};

let mut token = Token::new("workflow_id", StateId::from("initial"));
token.transition_to(StateId::from("target"), TransitionId::from("transition"));
```

**Option B: Rust GraphQL Client (distributed)**
```rust
// examples/rust/graphql_client.rs
let client = reqwest::Client::new();
let response = client.post("http://localhost:4000/graphql")
    .json(&create_token_query)
    .send().await?;
```

**Option C: Any Other Language**
```typescript
// examples/typescript/basic_workflow.ts
const client = new GraphQLClient('http://localhost:4000/graphql');
const result = await client.request(gql`mutation { ... }`);
```

## 🔬 Core Models API

### Generic Token Operations

```rust
use circuit_breaker::{Token, StateId, TransitionId};

// Create a token in any workflow
let mut token = Token::new("workflow_id", StateId::from("initial_state"));

// Set arbitrary data and metadata
token.data = serde_json::json!({
    "title": "My Item",
    "priority": "high"
});
token.set_metadata("department", serde_json::json!("engineering"));

// Transition to any state via any transition
token.transition_to(
    StateId::from("target_state"), 
    TransitionId::from("transition_name")
);

// Full audit trail automatically maintained
for event in &token.history {
    println!("{} -> {} via {}", 
        event.from.as_str(), 
        event.to.as_str(), 
        event.transition.as_str()
    );
}
```

### Workflow Definition and Validation

```rust
use circuit_breaker::{WorkflowDefinition, TransitionDefinition};

let workflow = WorkflowDefinition {
    id: "custom_process".to_string(),
    name: "Custom Process".to_string(),
    states: vec![
        StateId::from("start"),
        StateId::from("middle"), 
        StateId::from("end")
    ],
    transitions: vec![
        TransitionDefinition {
            id: TransitionId::from("begin"),
            from_states: vec![StateId::from("start")],
            to_state: StateId::from("middle"),
            conditions: vec!["validation_passed".to_string()],
        }
    ],
    initial_state: StateId::from("start"),
};

// Validate workflow structure
workflow.validate()?;

// Check valid transitions
let available = workflow.available_transitions(&StateId::from("start"));
```

## 🎛️ Engine & Server API

### GraphQL Engine

```rust
use circuit_breaker::{create_schema_with_storage, InMemoryStorage};

// Create GraphQL schema with storage backend
let storage = Box::new(InMemoryStorage::default());
let schema = create_schema_with_storage(storage);

// Available operations:
// - Query: workflows, tokens, availableTransitions
// - Mutation: createWorkflow, createToken, fireTransition  
// - Subscription: tokenUpdates, workflowEvents (coming soon)
```

### Production Server

```rust
use circuit_breaker::GraphQLServerBuilder;

// Development server with examples
let server = GraphQLServerBuilder::new()
    .port(4000)
    .build_with_examples()
    .await?;

// Production server with custom storage
let server = GraphQLServerBuilder::new()
    .host([0, 0, 0, 0])
    .port(8080) 
    .disable_playground()
    .disable_cors()
    .build_with_storage(production_storage);

server.start().await?;
```

## 🚀 Performance & Scalability

### Benchmarks

- **Token Creation**: ~100,000 tokens/second
- **State Transitions**: ~10,000 transitions/second  
- **Memory Usage**: <1MB per 10,000 active tokens
- **Startup Time**: <100ms cold start

### Distributed Architecture Ready

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Any Language   │    │  Any Language   │    │  Any Language   │
│     Client      │    │     Client      │    │     Client      │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
          ┌────────────────────────────────────────────┐
          │         🦀 Circuit Breaker Server          │
          │          cargo run --bin server            │
          │       http://localhost:4000/graphql        │
          └─────────────────┬──────────────────────────┘
                            │
          ┌────────────────────────────────────────────┐
          │            NATS JetStream Cluster         │
          │  ┌─────────┐ ┌─────────┐ ┌─────────────┐  │
          │  │ States  │ │ Tokens  │ │  Workflow   │  │
          │  │   KV    │ │ Stream  │ │ Definitions │  │
          │  └─────────┘ └─────────┘ └─────────────┘  │
          └────────────────────────────────────────────┘
```

## 🔄 Migration from DAG Systems

### From Airflow

```python
# Airflow - Linear DAG
@dag
def document_approval():
    draft = DummyOperator(task_id="draft")
    review = DummyOperator(task_id="review") 
    approve = DummyOperator(task_id="approve")
    
    draft >> review >> approve  # No cycles possible!
```

```graphql
# Circuit Breaker - State Managed Workflow
mutation {
  createWorkflow(input: {
    name: "Document Approval"
    states: ["draft", "review", "approved", "rejected"]
    transitions: [
      { id: "submit", fromStates: ["draft"], toState: "review" }
      { id: "approve", fromStates: ["review"], toState: "approved" }
      { id: "reject", fromStates: ["review"], toState: "rejected" }
      { id: "revise", fromStates: ["rejected"], toState: "draft" }  # Cycles supported!
    ]
  })
}
```

### From Temporal

```go
// Temporal - Procedural workflow
func DocumentWorkflow(ctx workflow.Context) error {
    // Linear execution - hard to model state relationships
    workflow.ExecuteActivity(ctx, DraftActivity)
    workflow.ExecuteActivity(ctx, ReviewActivity)
    workflow.ExecuteActivity(ctx, ApprovalActivity)
    return nil
}
```

```rust
// Circuit Breaker - Declarative state management  
let mut token = Token::new("document_workflow", StateId::from("draft"));
token.transition_to(StateId::from("review"), TransitionId::from("submit"));
// Rich state history and concurrent token support
```

## 🧠 AI Agent Campaign Use Cases

Circuit Breaker excels at coordinating complex AI agent workflows:

### Multi-Agent Content Pipeline

```graphql
mutation CreateContentCampaign {
  createWorkflow(input: {
    name: "AI Content Pipeline"
    states: [
      "research", "outline", "draft", "fact_check", 
      "edit", "review", "published", "promoted"
    ]
    transitions: [
      { id: "start_outline", fromStates: ["research"], toState: "outline" }
      { id: "start_draft", fromStates: ["outline"], toState: "draft" }
      { id: "fact_check", fromStates: ["draft"], toState: "fact_check" }
      { id: "needs_revision", fromStates: ["fact_check"], toState: "draft" }  # Revision loop
      { id: "approve_facts", fromStates: ["fact_check"], toState: "edit" }
      # ... more transitions
    ]
  })
}
```

**Agent Coordination Benefits:**
- **Parallel Processing**: Multiple agents can work on different tokens simultaneously
- **Revision Loops**: Natural support for agent feedback and iteration  
- **State Persistence**: Agents can pause/resume work with full context
- **Audit Trail**: Complete history of which agents performed what actions

## 🌍 Polyglot Client Examples

### Current Implementations

```bash
examples/
├── rust/              # 🦀 Rust clients
│   ├── basic_workflow.rs  # Direct model usage
│   ├── token_demo.rs      # Core operations demo  
│   ├── graphql_client.rs  # GraphQL client demo
│   └── mcp_cli.rs         # MCP CLI for authentication & OAuth
└── typescript/        # 📜 TypeScript clients  
    ├── basic_workflow.ts  # GraphQL client demo
    ├── token_demo.ts      # Token operations demo
    └── README.md          # Setup instructions
```

### Coming Soon

```bash
examples/
├── python/            # 🐍 Python clients (planned)
├── go/                # 🐹 Go clients (planned)  
├── java/              # ☕ Java clients (planned)
└── csharp/            # 🔷 C# clients (planned)
```

Each language directory will contain **client examples only**:
- `basic_workflow.*` - Architecture demonstration
- `token_demo.*` - Core operations demo
- `README.md` - Language-specific setup instructions

## 📚 Documentation

### Core Platform
- **[Executive Summary](docs/EXECUTIVE_SUMMARY.md)** - Complete platform overview and market positioning
- **[Circuit Breaker Server Guide](docs/CIRCUIT_BREAKER_SERVER_GUIDE.md)** - Comprehensive server setup and configuration
- **[NATS Comprehensive Guide](docs/NATS_COMPREHENSIVE_GUIDE.md)** - Distributed messaging and workflow persistence

### AI Integration & Providers
- **[OpenRouter Alternative](docs/OPENROUTER_ALTERNATIVE.md)** - BYOK LLM routing with multi-provider support
- **[Agent Configuration](docs/AGENT_CONFIGURATION.md)** - Multi-agent coordination and local AI integration
- **[MCP Tool Definitions](docs/MCP_TOOL_DEFINITIONS.md)** - Secure agent coordination and authentication

### Advanced Features
- **[Secure MCP Server](docs/SECURE_MCP_SERVER.md)** - GitHub Apps-style authentication for AI agents
- **[Function Runner](docs/FUNCTION_RUNNER.md)** - Containerized function execution with workflow integration
- **[Rules Engine](docs/RULES_ENGINE.md)** - Complex business logic evaluation and workflow transitions
- **[Webhook Integration Patterns](docs/WEBHOOK_INTEGRATION_PATTERNS.md)** - Event-driven workflows and external integrations

## 🤝 Contributing

```bash
# Setup development environment
git clone https://github.com/your-username/circuit-breaker
cd circuit-breaker
cargo test

# Start the server
cargo run --bin server

# Test client examples
cargo run --example basic_workflow
cargo run --example graphql_client
cargo run --example mcp_cli -- --help

# TypeScript examples
cd examples/typescript && npm install && npm run start:basic

# Visit playground
open http://localhost:4000/graphql
```

## 📈 Roadmap

### Phase 1: Core Engine (✅ Complete)
- [x] Generic token and workflow models  
- [x] GraphQL API with full CRUD operations
- [x] Production-ready server binary
- [x] Comprehensive client examples and documentation

### Phase 2: AI Integration & Local Support (✅ Complete)
- [x] NATS JetStream integration for persistence
- [x] Dynamic workflow stream creation
- [x] Token transitions via NATS messaging
- [x] Ollama integration with automatic model detection
- [x] Multi-provider LLM routing (OpenAI, Anthropic, Google, Azure)
- [x] Secure MCP server with GitHub Apps-style authentication
- [x] Real-time streaming with WebSocket and GraphQL subscriptions
- [x] Local AI embeddings and chat completions

### Phase 3: Advanced Agent Orchestration (🚧 In Progress)
- [x] AI agent orchestration and coordination
- [x] MCP (Model Context Protocol) integration
- [x] Multi-agent workflow coordination
- [x] Project-scoped AI operations
- [ ] Campaign management and monitoring
- [ ] Advanced workflow analytics
- [ ] Agent marketplace and templates

### Phase 4: Ecosystem (🔮 Future)
- [ ] Language-specific SDKs (Python, TypeScript, Go)
- [ ] Visual workflow designer
- [ ] Advanced Petri Net analysis tools
- [ ] Enterprise features and security

# Docs
```bash
cargo doc --open
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**Ready to build State Managed Workflows?** 🚀

```bash
# Start the server
cargo run --bin server

# Try client examples  
cargo run --example basic_workflow
cargo run --example mcp_cli -- interactive
cd examples/typescript && npm run start:basic

# Visit http://localhost:4000/graphql and start building!
``` 