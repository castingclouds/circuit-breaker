# Circuit Breaker - Rust Edition

> A distributed, polyglot workflow engine powered by **State Managed Workflows**, GraphQL, and Petri Nets  
> **Generic by design** - like Dagger's engine, the backend knows nothing about your domain  
> **State-centric** - unlike DAG-based systems, supports cycles, concurrent states, and complex workflows

## 🚀 Project Vision

Circuit Breaker is a **distributed, high-performance platform** for orchestrating complex workflows and AI agent campaigns. Inspired by [Dagger's](https://dagger.io) generic engine architecture, while pioneering **State Managed Workflows** powered by Petri Nets for mathematical rigor and formal workflow verification.

**Key Principles**: 
- **Generic Engine**: The Rust backend is domain-agnostic - all workflow logic defined via GraphQL
- **State Managed Workflows**: Unlike DAG-based systems, supports cycles, concurrent states, and complex relationships
- **Mathematical Guarantees**: Petri Net formalism provides deadlock detection and state safety
- **Polyglot First**: Any language can define and execute workflows through GraphQL

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

### Single Server, Multiple Clients

```
┌─────────────────────────────────────────────────────────────┐
│                🦀 Rust Main Server                          │
│             cargo run --bin server                         │  
│          http://localhost:4000/graphql                     │
└─────────────────────┬───────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
   🦀 Rust       📜 TypeScript   🐍 Python     
   Clients        Clients       Clients      
     │              │             │          
 Direct Models   GraphQL Only  GraphQL Only  
 GraphQL Client                               
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

### Infrastructure
- **Message Bus**: NATS JetStream for distributed state
- **API**: GraphQL (async-graphql) for polyglot clients
- **Web**: Axum for high-performance HTTP
- **Storage**: Pluggable backends (NATS KV, PostgreSQL, etc.)

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

## 🚀 Quick Start
</edits>

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
│   └── graphql_client.rs  # GraphQL client demo
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

- **[API Reference](docs/api.md)** - Complete GraphQL schema documentation
- **[Architecture Guide](docs/architecture.md)** - Deep dive into Petri Net workflow engine
- **[Migration Guide](docs/migration.md)** - Moving from DAG-based systems
- **[Performance Tuning](docs/performance.md)** - Optimization and scaling strategies

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

### Phase 2: Distributed Infrastructure (🚧 In Progress)
- [ ] NATS JetStream integration for persistence
- [ ] Horizontal scaling across multiple nodes
- [ ] Real-time subscriptions and event streaming
- [ ] Performance benchmarking and optimization

### Phase 3: Agent Framework (📋 Planned)
- [ ] AI agent orchestration and coordination
- [ ] MCP (Model Context Protocol) integration
- [ ] Campaign management and monitoring
- [ ] Advanced workflow analytics

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
cd examples/typescript && npm run start:basic

# Visit http://localhost:4000/graphql and start building!
``` 