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
npm run start:basic

# Core token operations  
npm run start:demo
```

## 📁 Client Examples

### `basic_workflow.ts` - **Polyglot Architecture Demo**

Shows how TypeScript defines workflows via GraphQL and executes them on the generic Rust backend.

**Key Features:**
- ✅ **TypeScript Application Development** workflow definition
- ✅ **Revision loops** and **cycles** (impossible with DAGs)
- ✅ **Client-side domain logic** sent to generic backend
- ✅ **GraphQL mutations** for workflow creation and token management

**Run:**
```bash
npm run start:basic
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
npm run start:demo
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
const client = new GraphQLClient('http://localhost:4000/graphql');

// Same API that Rust, Python, Go, Java use
const result = await client.request(gql`
  mutation CreateToken($input: TokenCreateInput!) {
    createToken(input: $input) { id state }
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

### 4. **Production Deployment**

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
npm run start:basic
npm run start:demo
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
- `README.md` - Language-specific setup instructions

## 💡 Why This Architecture?

### ✅ **Benefits:**
- **Single server**: One Rust binary to deploy and scale
- **Language flexibility**: Any language can be a client via GraphQL
- **API consistency**: Same interface across all languages
- **Performance**: Rust server handles the heavy lifting
- **Simplicity**: No multi-server coordination complexity

### ❌ **What We Avoided:**
- Multiple GraphQL servers in different languages
- Complex inter-service communication
- Language-specific performance bottlenecks
- Inconsistent APIs across languages

---

**Ready to build polyglot workflows?** 🚀

1. **Start server:** `cargo run --bin server` (in main directory)
2. **Install dependencies:** `npm install` (in this directory)
3. **Run client demos:** `npm run start:basic`
4. **Build your workflows** via GraphQL from **any language**! 🌐 