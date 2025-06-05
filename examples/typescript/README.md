# Circuit Breaker - TypeScript Client Examples

> **Client-only examples** demonstrating polyglot architecture  
> TypeScript clients → GraphQL → Generic Rust Backend

## 🌐 Polyglot Architecture Proof

These TypeScript examples prove that the **Rust backend is truly generic**:

- 🦀 **Rust Server**: One main binary (`cargo run --bin server`)
- 🌐 **GraphQL API**: Language-agnostic workflow definition and execution
- 📜 **TypeScript Clients**: Define domain-specific workflows through GraphQL
- 🔄 **State Management**: Same Petri Net guarantees regardless of client language

## 🚀 Quick Start

### 1. Start the Main Rust Server

In the main project directory:
```bash
# Option 1: In-memory storage (default)
cargo run --bin server

# Option 2: NATS storage (for distributed workflows)
export STORAGE_BACKEND=nats
export NATS_URL=nats://localhost:4222
cargo run --bin server
```

**Rust server will start at:** `http://localhost:4000/graphql`

### 2. Install TypeScript Dependencies

```bash
cd examples/typescript
npm install
```

### 3. Run TypeScript Client Examples

```bash
# Architecture demonstration
npm run demo:basic

# Core token operations  
npm run demo:token

# Function system demonstration
npm run demo:function

# Rules engine demonstration
npm run demo:rules

# Advanced GraphQL client features
npm run demo:graphql

# NATS integration demonstration
npm run demo:nats

# Or run the default function demo
npm run demo
```

## 📁 Complete Client Examples

### `basic_workflow.ts` - **Polyglot Architecture Demo**

Shows how TypeScript defines workflows via GraphQL and executes them on the generic Rust backend.

**Key Features:**
- ✅ **TypeScript Application Development** workflow definition
- ✅ **Revision loops** and **cycles** (impossible with DAGs)
- ✅ **Client-side domain logic** sent to generic backend
- ✅ **GraphQL mutations** for workflow creation and token management

**Run:**
```bash
npm run demo:basic
```

### `token_demo.ts` - **Core Token Operations**

Deep dive into token operations via GraphQL, showing TypeScript-specific data handling.

**Key Features:**
- ✅ **AI-Powered Content Creation** workflow with revision cycles
- ✅ **TypeScript-specific token data** (blog post metadata)
- ✅ **Client-side analysis** of workflow performance
- ✅ **History tracking** and duration calculations

**Run:**
```bash
npm run demo:token
```

### `function_demo.ts` - **Event-Driven Docker Functions**

Demonstrates the complete function system workflow with **REAL Docker container execution**.

**Key Features:**
- ✅ **Real Docker Execution**: Actually runs Docker containers, not simulation
- ✅ **Event-driven execution** based on workflow state changes
- ✅ **Live stdout/stderr capture** with real-time logging
- ✅ **Resource management** with memory, CPU, and timeout limits
- ✅ **Environment injection** of execution context automatically
- ✅ **Container lifecycle** management with automatic cleanup
- ✅ **Error handling** with proper exit codes and stderr capture
- ✅ **Output parsing** of JSON results from container execution

**Docker Features:**
- 🐳 **Container Management**: Full Docker container lifecycle
- 📊 **Real-time Monitoring**: Live capture of container stdout/stderr
- 🔒 **Security**: Resource limits prevent runaway containers
- 🗂️ **Environment Context**: Execution metadata injected automatically
- 🧹 **Auto-cleanup**: Containers removed after execution

**Run:**
```bash
npm run demo:function
```

**Example Output:**
```
🐳 Running Docker command: docker run --name circuit-breaker-abc123 --rm -e NODE_ENV=production...
📄 STDOUT: Processing order data...
📄 STDOUT: Input: {"orderId":"ORD-12345","total":1059.97}
📄 STDOUT: {"processed":true,"customerSegment":"high-value","recommendedShipping":"expedited"}
✅ Docker container completed successfully (exit code: 0)
```

### `rules_engine_demo.ts` - **Advanced Rules Engine**

Demonstrates sophisticated rule evaluation for token transitions with complex logical expressions.

**Key Features:**
- ✅ **Complex logical rules** with AND, OR, NOT operations  
- ✅ **Client-side rule evaluation** (with backend fallback)
- ✅ **Article publishing workflow** with quality gates
- ✅ **Emergency override scenarios** 
- ✅ **Detailed rule debugging** and evaluation feedback
- ✅ **Field-based conditions** (exists, equals, greater than, contains)

**Example Scenarios:**
- **Ready Article**: All quality criteria met
- **Incomplete Article**: Missing reviewer and content too short
- **Emergency Override**: Bypasses normal rules with emergency flag

**Run:**
```bash
npm run demo:rules
```

### `graphql_client.ts` - **Advanced GraphQL Client**

Shows direct GraphQL operations, performance testing, and advanced client features.

**Key Features:**
- ✅ **Schema introspection** and type discovery
- ✅ **Batch operations** for improved performance
- ✅ **Performance testing** and benchmarking
- ✅ **Request logging** and analytics
- ✅ **Production-ready client patterns**
- ✅ **Complex workflow** and token management

**Run:**
```bash
npm run demo:graphql
```

### `nats_demo.ts` - **NATS Integration Demo**

Demonstrates NATS JetStream storage backend integration with TypeScript clients.

**Key Features:**
- ✅ **NATS-enhanced GraphQL mutations** (`createWorkflowInstance`, `transitionTokenWithNats`)
- ✅ **Real-time token tracking** with NATS sequence numbers and subjects
- ✅ **Event-driven transitions** with NATS event publishing
- ✅ **Place-based token queries** optimized for NATS streams
- ✅ **Transition history** with NATS metadata tracking
- ✅ **TypeScript client library** for NATS-specific operations
- ✅ **Distributed storage backend** demonstration

**Prerequisites:**
```bash
# Start NATS server with JetStream
docker run -p 4222:4222 -p 8222:8222 nats:alpine --jetstream --http_port 8222

# Start Circuit Breaker server with NATS storage
export STORAGE_BACKEND=nats
export NATS_URL=nats://localhost:4222
cargo run --bin server
```

**Run:**
```bash
npm run demo:nats
```

## 🏗️ Architecture Comparison

### Direct Rust Usage (Server-Side)
```rust
// examples/rust/basic_workflow.rs - Direct model access
use circuit_breaker::{Token, StateId, WorkflowDefinition};

// Direct access to core models
let mut token = Token::new("workflow_id", StateId::from("initial"));
token.transition_to(StateId::from("target"), TransitionId::from("transition"));
```

### GraphQL Client Usage (Any Language)
```typescript
// examples/typescript/basic_workflow.ts - GraphQL client
const client = new CircuitBreakerClient('http://localhost:4000/graphql');

// Same API that Rust, Python, Go, Java use
const result = await client.request(gql`
  mutation CreateToken($input: TokenCreateInput!) {
    createToken(input: $input) { id place }
  }
`, { input: { workflowId: "any_workflow", data: {...} } });
```

```rust
// examples/rust/graphql_client.rs - Rust can also use GraphQL!
let client = reqwest::Client::new();
let response = client.post("http://localhost:4000/graphql")
    .json(&create_token_query)
    .send().await?;
```

## 🎯 Key Architectural Insights

### 1. **Single Source of Truth**
- **One Rust server binary**: `cargo run --bin server`
- **All clients**: Connect to the same GraphQL endpoint
- **No language-specific servers**: TypeScript, Python, Go are all clients

### 2. **True Language Agnostic**
The Rust backend has **zero client language knowledge**:
```
Client Language → GraphQL → Generic Rust Backend
     ↓               ↓              ↓
  TypeScript      Same API     Zero TypeScript
   Python        Same API     Zero Python  
     Go          Same API     Zero Go
    Java         Same API     Zero Java
```

### 3. **State Managed Workflows**
All languages work with the same **state management paradigm**:
- ✅ **Cycles supported** (revision loops, rollbacks)
- ✅ **Concurrent tokens** in different states
- ✅ **Complex transitions** with multiple sources
- ✅ **Mathematical guarantees** via Petri Nets

### 4. **Advanced Rules Engine**
The rules engine provides sophisticated condition evaluation:
- ✅ **Complex logical expressions**: `(A && B) || C` with arbitrary nesting
- ✅ **Field-based conditions**: Check metadata/data fields dynamically
- ✅ **Type-safe evaluation**: Rust backend ensures correctness
- ✅ **Client-side preview**: TypeScript can evaluate rules locally for UX

### 5. **Production Deployment**

**Simple & Correct:**
```
Multiple Language Clients → Single Rust Server (4000)
```

**Not This (what we had before):**
```
TypeScript Server (4001) → Rust Server (4000)  ❌ Unnecessary complexity
```

## 📊 Performance Benefits

| Approach | Latency | Memory | Complexity |
|----------|---------|---------|------------|
| **Direct Rust** | ~10μs | ~1KB | Low |
| **GraphQL Client** | ~2-5ms | ~10KB | Medium |
| **~~Multi-Server~~** | ~~>10ms~~ | ~~>50KB~~ | ~~High~~ |

**Conclusion:** Direct Rust for performance, GraphQL for flexibility, **no multi-server complexity**.

## 🔄 Development Workflow

### 1. **Start the Server**
```bash
# Main project directory
cargo run --bin server
```

### 2. **Develop Clients**
```bash
# TypeScript development (hot reload)  
cd examples/typescript
npm run dev

# Try different workflows via GraphQL
npm run demo:basic
npm run demo:token
npm run demo:function
npm run demo:rules
npm run demo:graphql
```

### 3. **Multi-Language Development**
- **Backend Team**: Focus on Rust server performance and generic capabilities
- **Client Teams**: Define domain workflows in their preferred language
- **GraphQL API**: Stable contract across all languages

## 🚀 Adding More Languages

```bash
examples/
├── rust/           # ✅ Server-side + GraphQL client examples
├── typescript/     # ✅ GraphQL client examples  
├── python/         # 🚧 Coming next - GraphQL clients only
├── go/             # 📋 Planned - GraphQL clients only
└── java/           # 📋 Planned - GraphQL clients only
```

Each language directory will contain **client examples only**:
- `basic_workflow.*` - Architecture demonstration
- `token_demo.*` - Core operations demo
- `function_demo.*` - Event-driven function system
- `rules_engine_demo.*` - Advanced rules evaluation
- `graphql_client.*` - Advanced client features
- `README.md` - Language-specific setup instructions

## 💡 Why This Architecture?

### ✅ **Benefits:**
- **Single server**: One Rust binary to deploy and scale
- **Language flexibility**: Any language can be a client via GraphQL
- **API consistency**: Same interface across all languages
- **Rules flexibility**: Complex business logic without hardcoded conditions
- **Type safety**: Rust backend prevents rule evaluation errors
- **Client preview**: Languages can implement local rule evaluation for better UX

### ❌ **Anti-patterns avoided:**
- Multiple language-specific servers
- Hardcoded business logic in core engine
- Complex deployment coordination
- API inconsistencies between languages

## 🤖 Rules Engine Deep Dive

The rules engine enables sophisticated workflow control without hardcoding business logic:

### Rule Types
```typescript
// Field existence
RuleBuilder.fieldExists('has_reviewer', 'reviewer')

// Value equality  
RuleBuilder.fieldEquals('status_approved', 'status', 'approved')

// Numeric comparisons
RuleBuilder.fieldGreaterThan('high_priority', 'priority', 5)

// String operations
RuleBuilder.fieldContains('urgent_tag', 'tags', 'urgent')

// Complex logical expressions
RuleBuilder.or('publish_ready', 'Ready to publish', [
  RuleBuilder.and('quality_criteria', 'High quality', [
    RuleBuilder.fieldExists('has_content', 'content'),
    RuleBuilder.fieldEquals('status_approved', 'status', 'approved'),
    RuleBuilder.fieldGreaterThan('word_count_sufficient', 'word_count', 500)
  ]),
  RuleBuilder.fieldEquals('emergency_flag', 'emergency', true)
])
```

### Integration Benefits
- **Domain Agnostic**: Rules work with any JSON metadata/data
- **Debuggable**: Detailed evaluation results show why rules pass/fail
- **Composable**: Complex expressions built from simple components
- **Reusable**: Common rules shared across workflows
- **Type Safe**: Rust backend prevents evaluation errors

## 🔧 Client Development Tips

### 1. **Start Simple**
```typescript
// Begin with basic workflows
const workflow = {
  places: ['draft', 'review', 'published'],
  transitions: [
    { id: 'submit', fromPlaces: ['draft'], toPlace: 'review' },
    { id: 'publish', fromPlaces: ['review'], toPlace: 'published' }
  ]
};
```

### 2. **Add Rules Gradually**
```typescript
// Add rules as business logic becomes clear
transition.rules = [
  RuleBuilder.fieldExists('has_content', 'content'),
  RuleBuilder.fieldGreaterThan('quality_score', 'score', 8)
];
```

### 3. **Use Client-Side Preview**
```typescript
// Implement local rule evaluation for immediate UI feedback
const canTransition = clientEngine.canTransition(token, transition);
// Then validate on server for authoritative result
```

### 4. **Leverage Type Safety**
```typescript
// Define strong types for your domain
interface ArticleData {
  title: string;
  content: string;
  wordCount: number;
  tags: string[];
}

// Use with token data
const token: Token & { data: ArticleData } = ...;
```

## 🎨 Client Feature Showcase

The TypeScript examples demonstrate advanced client capabilities:

### `CircuitBreakerClient`
- **GraphQL Operations**: Simplified GraphQL query/mutation execution
- **Type Safety**: Full TypeScript interfaces for all data structures
- **Error Handling**: Proper error handling for network and GraphQL errors
- **Workflow Management**: Create workflows, tokens, and fire transitions

### `AdvancedGraphQLClient`
- **Performance Testing**: Built-in benchmarking utilities
- **Request Logging**: Complete request analytics and debugging
- **Batch Operations**: Parallel execution for improved performance
- **Schema Introspection**: Runtime schema discovery

### `RuleBuilder` & `ClientRuleEngine`
- **Rule Construction**: Fluent API for building complex conditions
- **Client Evaluation**: Immediate feedback without server round-trips
- **Debugging Support**: Detailed evaluation results and failure reasons

## 📈 Comparison with Rust Examples

| Feature | Rust Examples | TypeScript Examples |
|---------|---------------|-------------------|
| **Basic Workflows** | ✅ `basic_workflow.rs` | ✅ `basic_workflow.ts` |
| **Token Operations** | ✅ `token_demo.rs` | ✅ `token_demo.ts` |
| **Function System** | ✅ `function_demo.rs` | ✅ `function_demo.ts` |
| **Rules Engine** | ✅ `rules_engine_demo.rs` | ✅ `rules_engine_demo.ts` |
| **GraphQL Client** | ✅ `graphql_client.rs` | ✅ `graphql_client.ts` |
| **Performance** | Direct model access | GraphQL API |
| **Type Safety** | Rust type system | TypeScript interfaces |
| **Execution** | Server-side | Client-side |
| **Development** | `cargo run --example` | `npm run demo:*` |

## 🛠️ Development Environment Setup

### TypeScript Development
```bash
# Type checking
npm run type-check

# Run specific examples
npm run demo:basic     # Basic workflow
npm run demo:token     # Token operations
npm run demo:function  # Function system
npm run demo:rules     # Rules engine
npm run demo:graphql   # Advanced client

# Explore GraphQL interactively
open http://localhost:4000  # GraphiQL interface
```

### Server Integration
1. **Start Server**: `cargo run --bin server` (project root)
2. **Wait for Startup**: Server logs will show GraphQL endpoints
3. **Run Examples**: Any TypeScript demo in `examples/typescript/`
4. **Monitor**: Check server logs for GraphQL operations

## 🚀 Future Enhancements

Planned improvements across all client languages:

### GraphQL API Extensions
- **Function Management**: CRUD operations for Docker functions
- **Real-time Subscriptions**: Live workflow and function updates
- **Advanced Queries**: Complex filtering and pagination
- **Batch Mutations**: Efficient bulk operations

### Client Libraries
- **Code Generation**: Auto-generate clients from GraphQL schema
- **Framework Integrations**: React hooks, Vue composables, etc.
- **Caching Strategies**: Apollo Client, React Query integration
- **Offline Support**: Local-first architecture patterns

### Developer Experience
- **Visual Workflow Editor**: Drag-and-drop workflow designer
- **Function Templates**: Pre-built function configurations
- **Debugging Tools**: Visual rule evaluation and workflow tracing
- **Performance Monitoring**: Client-side metrics and analytics

---

**Ready to build polyglot workflows?** 🚀

1. **Start server:** `cargo run --bin server` (in main directory)
2. **Install dependencies:** `npm install` (in this directory)
3. **Run client demos:** `npm run demo:basic`
4. **Build your workflows** via GraphQL from **any language**! 🌐
