# Circuit Breaker Standalone Agent Architecture - Product Enhancement Document

## 1. Executive Summary

### 1.1 Overview
This document outlines the refactoring of Circuit Breaker's agent capabilities from a workflow-coupled system to a standalone, reusable agent architecture. The new design decouples agents from Petri net workflows while maintaining integration capabilities through a bridge layer.

### 1.2 Current State Problems
- Agents are tightly coupled to workflow `Resource` and `StateId` types
- Agent execution requires workflow engine context
- Cannot run agents independently for testing or external integration
- MCP capabilities are separate from agent capabilities
- Limited reusability across different contexts

### 1.3 Proposed Solution
Create a standalone agent architecture with:
- Generic context-based execution model
- Workflow integration bridge layer
- MCP protocol adapter
- Multi-tenant isolation
- Comprehensive testing framework

## 2. Architecture Overview

### 2.1 High-Level Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                 External Interfaces                         │
├─────────────┬─────────────┬─────────────┬─────────────────┤
│   HTTP API  │   MCP       │  GraphQL    │   WebSocket     │
│   Adapter   │   Adapter   │   Adapter   │   Adapter       │
└─────────────┴─────────────┴─────────────┴─────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                Standalone Agent Engine                      │
├─────────────────────────────────────────────────────────────┤
│  • Context-based execution                                  │
│  • Generic input/output mapping                             │
│  • LLM provider abstraction                                 │
│  • Streaming & retry logic                                  │
│  • Multi-tenant isolation                                   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│              Integration Bridge Layer                       │
├─────────────────────────────────────────────────────────────┤
│  • Workflow → Agent context mapping                         │
│  • Agent → Workflow result application                      │
│  • Petri net state management                              │
│  • Resource lifecycle integration                           │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Core Components
- **Standalone Agent Engine**: Core execution logic independent of workflows
- **Context Abstraction**: Generic execution context replacing Resource/State dependencies
- **Integration Bridge**: Workflow-specific adapter layer
- **Protocol Adapters**: MCP, HTTP, GraphQL, WebSocket interfaces
- **Multi-Tenant Storage**: Isolated agent data and configurations

## 3. Implementation Roadmap

### 3.1 Phase 1: Core Agent Refactoring (Weeks 1-2)

#### 3.1.1 Remove Workflow Dependencies from Agent Models
**Target Files**: `src/models/agent.rs`

**Tasks**:
- [x] **Prompt**: Remove `resource_id: Uuid` and `state_id: StateId` from `AgentExecution` struct
- [x] **Prompt**: Replace with `context: serde_json::Value` field for generic metadata
- [x] **Prompt**: Update `AgentExecution::new()` constructor to accept context instead of resource_id/state_id
- [x] **Prompt**: Add helper methods for context manipulation: `get_context_value()`, `set_context_value()`
- [x] **Prompt**: Remove `StateAgentConfig` struct or make it optional/workflow-specific
- [x] **Prompt**: Update `AgentActivityConfig` to use generic context mappings
- [x] **Compile Test**: Ensure `src/models/agent.rs` compiles independently

#### 3.1.2 Abstract Agent Storage Interface
**Target Files**: `src/engine/agents.rs` (storage trait)

**Tasks**:
- [ ] **Prompt**: Remove `StateAgentConfig` references from `AgentStorage` trait
- [ ] **Prompt**: Update `store_execution()` and related methods to use generic context
- [ ] **Prompt**: Add `list_executions_by_context()` method for flexible querying
- [ ] **Prompt**: Update `InMemoryAgentStorage` implementation to match new interface
- [ ] **Prompt**: Add context-based indexing for efficient queries
- [ ] **Compile Test**: Ensure storage trait compiles and basic tests pass

#### 3.1.3 Refactor Agent Engine Core
**Target Files**: `src/engine/agents.rs` (engine implementation)

**Tasks**:
- [ ] **Prompt**: Remove `RulesEngine` dependency from `AgentEngine` constructor
- [ ] **Prompt**: Update `execute_agent_internal()` to accept generic context instead of Resource
- [ ] **Prompt**: Refactor `map_input_data()` to work with `serde_json::Value` context
- [ ] **Prompt**: Update `apply_output_to_resource()` to `apply_output_to_context()`
- [ ] **Prompt**: Remove workflow-specific validation logic
- [ ] **Prompt**: Update streaming to use context-based metadata
- [ ] **Compile Test**: Ensure core agent engine compiles independently

### 3.2 Phase 2: Create Integration Bridge Layer (Weeks 2-3)

#### 3.2.1 Design Workflow Integration Interface
**Target Files**: `src/integration/workflow_bridge.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `WorkflowAgentBridge` struct with workflow storage reference
- [ ] **Prompt**: Implement `resource_to_context()` method for Resource → context mapping
- [ ] **Prompt**: Implement `context_to_resource()` method for result application
- [ ] **Prompt**: Add `execute_workflow_agent()` method that handles full workflow integration
- [ ] **Prompt**: Include state transition logic based on agent results
- [ ] **Prompt**: Add error handling for workflow-specific failures
- [ ] **Compile Test**: Ensure bridge layer compiles and basic workflow tests pass

#### 3.2.2 Implement State Agent Bridge
**Target Files**: `src/integration/state_agent_bridge.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `StateAgentBridge` for managing place-based agent execution
- [ ] **Prompt**: Implement state monitoring and agent triggering logic
- [ ] **Prompt**: Add scheduling support for periodic agent execution
- [ ] **Prompt**: Include rule evaluation for agent trigger conditions
- [ ] **Prompt**: Update to use context-based agent execution
- [ ] **Compile Test**: Ensure state agent bridge works with existing workflow engine

#### 3.2.3 Update Workflow Engine Integration
**Target Files**: `src/engine/mod.rs`, workflow engine files

**Tasks**:
- [ ] **Prompt**: Update workflow engine to use `WorkflowAgentBridge` instead of direct agent calls
- [ ] **Prompt**: Remove direct `AgentEngine` references from workflow code
- [ ] **Prompt**: Update GraphQL resolvers to use bridge layer
- [ ] **Prompt**: Modify activity execution to delegate to bridge layer
- [ ] **Prompt**: Update NATS integration to work with new bridge
- [ ] **Integration Test**: Ensure existing workflow functionality works with bridge layer

### 3.3 Phase 3: Standalone Agent API (Weeks 3-4)

#### 3.3.1 Design Standalone Agent API
**Target Files**: `src/agents/api.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `StandaloneAgentApi` struct with agent engine reference
- [ ] **Prompt**: Implement `execute_agent()` method with generic context input
- [ ] **Prompt**: Add streaming support with SSE/WebSocket protocols
- [ ] **Prompt**: Include tenant isolation in API layer
- [ ] **Prompt**: Add comprehensive error handling and validation
- [ ] **Prompt**: Implement rate limiting and request queuing
- [ ] **Compile Test**: Ensure standalone API compiles and basic tests pass

#### 3.3.2 HTTP REST Endpoints
**Target Files**: `src/agents/http_handlers.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `POST /agents/{agent_id}/execute` endpoint
- [ ] **Prompt**: Create `GET /agents/{agent_id}/executions` endpoint
- [ ] **Prompt**: Create `GET /agents/{agent_id}/executions/{execution_id}` endpoint
- [ ] **Prompt**: Create `GET /agents/{agent_id}/executions/{execution_id}/stream` SSE endpoint
- [ ] **Prompt**: Add request/response validation and serialization
- [ ] **Prompt**: Include tenant-aware routing and authentication
- [ ] **Integration Test**: Test all HTTP endpoints with real agent execution

#### 3.3.3 WebSocket Streaming Support
**Target Files**: `src/agents/websocket_handlers.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create WebSocket handler for real-time agent streaming
- [ ] **Prompt**: Implement connection management and authentication
- [ ] **Prompt**: Add message routing for different execution contexts
- [ ] **Prompt**: Include error handling and connection recovery
- [ ] **Prompt**: Add support for multiple concurrent executions per connection
- [ ] **Integration Test**: Test WebSocket streaming with long-running agent tasks

### 3.4 Phase 4: MCP Protocol Integration (Weeks 4-5)

#### 3.4.1 MCP Agent Tools Adapter
**Target Files**: `src/agents/mcp_adapter.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `MCPAgentAdapter` that exposes agents as MCP tools
- [ ] **Prompt**: Implement tool discovery and registration
- [ ] **Prompt**: Map agent execution to MCP tool calls
- [ ] **Prompt**: Handle MCP-specific authentication and session management
- [ ] **Prompt**: Add support for tool streaming and progress updates
- [ ] **Compile Test**: Ensure MCP adapter compiles and registers tools correctly

#### 3.4.2 MCP Resources Integration
**Target Files**: `src/agents/mcp_resources.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Expose agent executions as MCP resources
- [ ] **Prompt**: Implement resource querying and filtering
- [ ] **Prompt**: Add support for execution history and metadata
- [ ] **Prompt**: Include real-time resource updates via MCP subscriptions
- [ ] **Prompt**: Add tenant isolation for MCP resource access
- [ ] **Integration Test**: Test MCP resource access from external MCP clients

#### 3.4.3 Update Existing MCP Server
**Target Files**: `src/api/mcp_server.rs`

**Tasks**:
- [ ] **Prompt**: Integrate `MCPAgentAdapter` into existing MCP server
- [ ] **Prompt**: Add agent tool registration to MCP server initialization
- [ ] **Prompt**: Update MCP server to handle agent-specific requests
- [ ] **Prompt**: Add agent execution monitoring to MCP server
- [ ] **Prompt**: Include agent tools in MCP server capabilities
- [ ] **Integration Test**: Test full MCP server with agent integration

### 3.5 Phase 5: Multi-Tenant Architecture (Weeks 5-6)

#### 3.5.1 Tenant Isolation in Agent Engine
**Target Files**: `src/agents/tenant_isolation.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `TenantAwareAgentEngine` wrapper around standalone engine
- [ ] **Prompt**: Implement tenant-specific agent storage and configuration
- [ ] **Prompt**: Add tenant-based resource limits and quotas
- [ ] **Prompt**: Include tenant-specific authentication and authorization
- [ ] **Prompt**: Add tenant isolation for agent execution contexts
- [ ] **Compile Test**: Ensure tenant isolation compiles and basic tests pass

#### 3.5.2 Multi-Tenant Storage Layer
**Target Files**: `src/agents/tenant_storage.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `TenantAgentStorage` implementation
- [ ] **Prompt**: Add tenant-specific data partitioning
- [ ] **Prompt**: Implement tenant-aware querying and filtering
- [ ] **Prompt**: Add tenant-specific backup and recovery
- [ ] **Prompt**: Include tenant usage analytics and monitoring
- [ ] **Integration Test**: Test multi-tenant storage with concurrent tenants

#### 3.5.3 Authentication and Authorization
**Target Files**: `src/agents/auth.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create tenant-aware authentication middleware
- [ ] **Prompt**: Implement role-based access control for agents
- [ ] **Prompt**: Add API key and JWT token validation
- [ ] **Prompt**: Include tenant-specific rate limiting
- [ ] **Prompt**: Add audit logging for tenant operations
- [ ] **Integration Test**: Test authentication with multiple tenants and roles

### 3.6 Phase 6: Advanced Features (Weeks 6-7)

#### 3.6.1 Agent Composition and Chaining
**Target Files**: `src/agents/composition.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Create `AgentComposer` for chaining multiple agents
- [ ] **Prompt**: Implement context passing between chained agents
- [ ] **Prompt**: Add conditional execution based on previous agent results
- [ ] **Prompt**: Include error handling and fallback strategies
- [ ] **Prompt**: Add parallel execution support for independent agents
- [ ] **Compile Test**: Ensure agent composition compiles and basic tests pass

#### 3.6.2 Performance Optimization
**Target Files**: `src/agents/performance.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Implement connection pooling for LLM providers
- [ ] **Prompt**: Add caching for frequently used agent contexts
- [ ] **Prompt**: Include request batching and queuing optimization
- [ ] **Prompt**: Add execution time monitoring and optimization
- [ ] **Prompt**: Implement resource usage tracking and limits
- [ ] **Performance Test**: Benchmark agent execution under load

#### 3.6.3 Advanced Streaming Features
**Target Files**: `src/agents/streaming.rs` (new file)

**Tasks**:
- [ ] **Prompt**: Implement chunked streaming for large responses
- [ ] **Prompt**: Add progress tracking and percentage completion
- [ ] **Prompt**: Include streaming backpressure handling
- [ ] **Prompt**: Add support for streaming cancellation
- [ ] **Prompt**: Implement stream multiplexing for multiple agents
- [ ] **Integration Test**: Test streaming under various network conditions

### 3.7 Phase 7: Testing and Documentation (Weeks 7-8)

#### 3.7.1 Comprehensive Unit Testing
**Target Files**: `src/agents/tests/` (new directory)

**Tasks**:
- [ ] **Prompt**: Create unit tests for standalone agent engine
- [ ] **Prompt**: Add tests for context manipulation and mapping
- [ ] **Prompt**: Include tests for all LLM provider integrations
- [ ] **Prompt**: Add tests for streaming and error handling
- [ ] **Prompt**: Include tests for multi-tenant isolation
- [ ] **Prompt**: Add performance and load testing
- [ ] **Test Coverage**: Achieve >90% test coverage for agent modules

#### 3.7.2 Integration Testing
**Target Files**: `tests/integration/` (new directory)

**Tasks**:
- [ ] **Prompt**: Create integration tests for workflow bridge layer
- [ ] **Prompt**: Add tests for MCP protocol integration
- [ ] **Prompt**: Include tests for HTTP API endpoints
- [ ] **Prompt**: Add tests for WebSocket streaming
- [ ] **Prompt**: Include tests for multi-tenant scenarios
- [ ] **Integration Test**: All integration tests pass consistently

#### 3.7.3 Documentation and Examples
**Target Files**: `docs/agents/` (new directory)

**Tasks**:
- [ ] **Prompt**: Create standalone agent API documentation
- [ ] **Prompt**: Add workflow integration guide
- [ ] **Prompt**: Include MCP integration examples
- [ ] **Prompt**: Add multi-tenant setup guide
- [ ] **Prompt**: Include performance tuning recommendations
- [ ] **Prompt**: Add troubleshooting guide
- [ ] **Documentation Review**: All documentation reviewed and approved

## 4. API Specifications

### 4.1 Standalone Agent Execution API

#### 4.1.1 Execute Agent (Non-Streaming)
```http
POST /agents/{agent_id}/execute
Content-Type: application/json
Authorization: Bearer {tenant_token}

{
  "context": {
    "user_id": "user123",
    "session_id": "session456",
    "custom_data": {
      "key": "value"
    }
  },
  "input": {
    "message": "Analyze this data...",
    "parameters": {
      "temperature": 0.7,
      "max_tokens": 1000
    }
  },
  "execution_config": {
    "timeout_seconds": 300,
    "retry_config": {
      "max_attempts": 3,
      "backoff_seconds": 10
    }
  }
}
```

#### 4.1.2 Execute Agent (Streaming)
```http
POST /agents/{agent_id}/execute/stream
Content-Type: application/json
Authorization: Bearer {tenant_token}
Accept: text/event-stream

{
  "context": { ... },
  "input": { ... },
  "execution_config": {
    "stream": true,
    "stream_format": "sse"
  }
}
```

### 4.2 Context Structure

#### 4.2.1 Generic Context Format
```json
{
  "tenant_id": "tenant123",
  "execution_id": "exec456",
  "user_context": {
    "user_id": "user123",
    "session_id": "session456",
    "preferences": {}
  },
  "workflow_context": {
    "resource_id": "resource789",
    "state_id": "state_pending",
    "transition_id": "transition123"
  },
  "custom_context": {
    "any_key": "any_value"
  }
}
```

#### 4.2.2 Workflow Bridge Context Mapping
```rust
// Example of how workflow bridge maps Resource to context
impl WorkflowAgentBridge {
    fn resource_to_context(&self, resource: &Resource) -> serde_json::Value {
        json!({
            "tenant_id": self.tenant_id,
            "workflow_context": {
                "resource_id": resource.id,
                "state_id": resource.current_state,
                "metadata": resource.metadata
            },
            "user_context": {
                "user_id": resource.metadata.get("user_id"),
                "session_id": resource.metadata.get("session_id")
            }
        })
    }
}
```

## 5. Multi-Tenant Architecture

### 5.1 Tenant Isolation Strategy
- **Data Isolation**: Tenant-specific databases/schemas
- **Execution Isolation**: Tenant-specific agent pools
- **Resource Isolation**: Per-tenant quotas and limits
- **Configuration Isolation**: Tenant-specific agent configurations

### 5.2 Authentication and Authorization
- **Tenant Tokens**: JWT tokens with tenant claims
- **Role-Based Access**: Admin, user, readonly roles per tenant
- **API Key Support**: Alternative authentication method
- **Audit Logging**: All tenant operations logged

## 6. Error Handling and Observability

### 6.1 Error Handling Strategy
- **Structured Errors**: Consistent error format across all APIs
- **Error Recovery**: Automatic retry with exponential backoff
- **Graceful Degradation**: Fallback to simpler models on failure
- **Error Aggregation**: Collect and report error patterns

### 6.2 Logging and Monitoring
- **Structured Logging**: JSON format with correlation IDs
- **Metrics Collection**: Execution times, success rates, resource usage
- **Distributed Tracing**: Request flow across all components
- **Alerting**: Automated alerts for failures and performance issues

## 7. Success Metrics

### 7.1 Technical Metrics
- **Compilation Success**: All phases compile successfully
- **Test Coverage**: >90% unit test coverage
- **Performance**: <100ms API response time for simple executions
- **Reliability**: >99.9% uptime for agent execution

### 7.2 Business Metrics
- **Developer Adoption**: Standalone API usage metrics
- **Integration Success**: Successful workflow and MCP integrations
- **Multi-Tenant Usage**: Number of active tenants
- **Error Rates**: <1% execution failure rate

## 8. Migration Strategy

### 8.1 Backward Compatibility
- **Bridge Layer**: Existing workflow integration continues to work
- **Gradual Migration**: Phase-by-phase rollout with rollback capability
- **Feature Flags**: Toggle between old and new implementations
- **Deprecation Timeline**: 6-month deprecation period for old APIs

### 8.2 Data Migration
- **Context Mapping**: Automatic migration of existing agent executions
- **Configuration Update**: Migrate existing agent configurations
- **Storage Migration**: Migrate to new multi-tenant storage format
- **Validation**: Extensive testing during migration

## 9. Security Considerations

### 9.1 Data Protection
- **Encryption**: All data encrypted at rest and in transit
- **PII Handling**: Proper handling of personally identifiable information
- **Data Retention**: Configurable retention policies per tenant
- **Access Controls**: Fine-grained access control for sensitive operations

### 9.2 Rate Limiting and Abuse Prevention
- **Per-Tenant Limits**: Configurable rate limits per tenant
- **Resource Quotas**: CPU, memory, and execution time limits
- **Abuse Detection**: Automated detection and blocking of abusive patterns
- **Circuit Breakers**: Automatic fallback on system overload

## 10. Conclusion

This refactoring transforms Circuit Breaker's agent capabilities from a workflow-coupled system to a standalone, reusable architecture. The detailed implementation plan with checkboxes ensures systematic progress while maintaining the ability to compile and test at each stage. The new architecture provides:

- **Flexibility**: Agents can be used in any context
- **Testability**: Comprehensive testing without workflow dependencies
- **Scalability**: Multi-tenant architecture with proper isolation
- **Maintainability**: Clear separation of concerns and modular design
- **Integration**: Seamless integration with existing workflow engine through bridge layer

The phased approach ensures minimal disruption while providing maximum benefit for future development and third-party integrations.