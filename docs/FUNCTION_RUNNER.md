# Function Runner Documentation

The Circuit Breaker Function Runner provides event-driven, chainable Docker-based functions that execute in response to workflow events. Functions can process data, perform external operations, and trigger other functions through a sophisticated chaining system.

## Overview

The Function Runner enables serverless-style execution within Circuit Breaker workflows. Functions are triggered by events (token creation, transitions, completions), execute in isolated Docker containers, and can chain together to create complex data processing pipelines.

## Implementation Status

### ✅ **Currently Working Features**

- **Docker Execution Engine**: Full Docker container execution with real-time output streaming
- **Event Processing**: Functions are triggered by workflow events
- **Container Lifecycle Management**: Automatic container creation, execution, and cleanup
- **Input/Output Processing**: Event data mapping to function inputs and output parsing
- **Execution Tracking**: Complete execution records with status, timing, and results
- **Error Handling**: Comprehensive error capture and reporting
- **Resource Management**: Docker container resource limits and isolation
- **Real-time Logging**: Live stdout/stderr capture during execution
- **Function Storage**: In-memory storage implementation for functions and executions
- **Docker Availability Detection**: Automatic detection of Docker availability

### 🚧 **Partially Implemented**

- **Function Chaining**: Core logic implemented but temporarily disabled due to lifetime issues
- **Retry Mechanisms**: Logic implemented but scheduling system needs refinement
- **Input Validation**: JSON Schema validation framework in place, needs integration

### 📋 **Planned Features**

- **JSON Schema Integration**: Full input/output validation
- **GraphQL API**: Function management and monitoring API
- **Secret Management**: Secure credential injection
- **Container Optimization**: Image caching and reuse strategies
- **Advanced Chaining**: Complex condition evaluation and template mapping

## Key Features

- **Event-Driven Execution**: Functions trigger automatically based on workflow events
- **Docker Isolation**: Each function runs in its own Docker container with resource limits
- **Function Chaining**: Outputs from one function can trigger and provide input to other functions
- **Schema Validation**: Structured input/output schemas with JSON Schema validation
- **Rules Engine Integration**: Function outputs available as data in workflow rules
- **Real-time Monitoring**: Live execution tracking with stdout/stderr capture
- **Async Execution**: Non-blocking function execution with result tracking
- **Error Handling**: Comprehensive error tracking and retry mechanisms

## Architecture Components

### 1. Function Definition (`FunctionDefinition`)

A function definition contains:
- **Metadata**: ID, name, description, creation timestamps
- **Container Configuration**: Docker image, commands, environment, resources
- **Input Schema**: JSON Schema defining expected input structure
- **Output Schema**: JSON Schema defining expected output structure
- **Event Triggers**: What events cause this function to execute
- **Chaining Rules**: How outputs trigger other functions
- **Timeout and Retry Settings**: Execution limits and failure handling

```rust
FunctionDefinition {
    id: FunctionId("process_document"),
    name: "Document Processor",
    description: "Extracts metadata from uploaded documents",
    container: ContainerConfig {
        image: "document-processor:latest",
        exec_command: vec!["node", "index.js"],
        env_vars: {"NODE_ENV": "production"},
        resources: ResourceLimits {
            memory_mb: 512,
            cpu_cores: 0.5,
            timeout_seconds: 300,
        }
    },
    input_schema: FunctionSchema {
        schema: json!({
            "type": "object",
            "required": ["document_url", "document_type"],
            "properties": {
                "document_url": {"type": "string", "format": "uri"},
                "document_type": {"type": "string", "enum": ["pdf", "docx", "txt"]}
            }
        })
    },
    output_schema: FunctionSchema {
        schema: json!({
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "author": {"type": "string"},
                "page_count": {"type": "integer"},
                "extracted_text": {"type": "string"},
                "keywords": {"type": "array", "items": {"type": "string"}}
            }
        })
    },
    triggers: vec![
        EventTrigger::on_token_created("document_uploaded", Some(PlaceId::from("uploaded")))
    ],
    chains: vec![
        FunctionChain {
            target_function: FunctionId("sentiment_analysis"),
            condition: ChainCondition::Always,
            input_mapping: OutputMapping::field("extracted_text", "text"),
        }
    ]
}
```

### 2. Docker Execution Engine (`FunctionEngine`)

The execution engine handles:
- **Container Management**: Full Docker container lifecycle with automatic cleanup
- **Input Preparation**: Event data mapping and validation 
- **Execution Monitoring**: Real-time stdout/stderr capture with logging
- **Output Processing**: JSON output parsing and validation
- **Error Handling**: Comprehensive failure capture and reporting
- **Storage Integration**: Persistent execution records and function definitions

#### Current Docker Implementation Features:

- **Real-time Output Streaming**: Live capture of container stdout/stderr
- **Environment Variable Injection**: Automatic injection of execution context
- **Resource Limits**: Memory, CPU, and timeout constraints
- **Volume Mounting**: File system mounts for data access
- **Multi-command Support**: Setup commands and main execution commands
- **Automatic Cleanup**: Container removal after execution completion

```rust
// Docker execution with real-time logging
🔧 Running Docker command: docker run --name circuit-breaker-abc123 --rm -e NODE_ENV=production -e EXECUTION_ID=abc123 -e INPUT_DATA={"file":"test.pdf"} node:18-alpine node process.js
📦 Starting Docker container...
📄 STDOUT: Processing file: test.pdf
📄 STDOUT: Extracting text content...
📄 STDOUT: {"title": "Sample Document", "pages": 5, "text": "..."}
✅ Docker container completed successfully (exit code: 0)
```

### 3. Event System (`EventBus`)

The event bus manages function triggering:
- **Event Publishing**: Workflow operations publish events (token created, transitioned, etc.)
- **Function Matching**: Determine which functions should run for each event
- **Execution Queuing**: Queue function executions for processing
- **Result Broadcasting**: Publish function completion events for chaining

#### Event Types

- `TokenCreated`: New token created in a place
- `TokenTransitioned`: Token moved between places
- `TokenUpdated`: Token metadata/data changed
- `TokenCompleted`: Token reached final state
- `FunctionCompleted`: Function finished execution (for chaining)
- `WorkflowCreated`: New workflow definition created
- `Custom`: Application-specific events

### 4. Function Chaining System (Partially Implemented)

Functions can trigger other functions based on completion:

#### Chain Conditions
- `Always`: Always trigger next function
- `OnSuccess`: Only trigger if function succeeded
- `OnFailure`: Only trigger if function failed
- `ConditionalRule`: Use rules engine to determine triggering

#### Input Mapping
- `FullOutput`: Pass entire output as input
- `FieldMapping`: Map specific output fields to input fields
- `TemplateMapping`: Use templates to transform data
- `MergedData`: Combine function output with token data

```rust
FunctionChain {
    target_function: FunctionId("send_notification"),
    condition: ChainCondition::ConditionalRule(
        Rule::field_equals("priority", "priority", json!("high"))
    ),
    input_mapping: InputMapping::Template(json!({
        "message": "Document '{title}' processed successfully",
        "recipient": "{$.token.metadata.user_email}",
        "priority": "{$.output.priority}"
    })),
    delay: Some(Duration::seconds(30)), // Optional delay before triggering
}
```

### 5. Storage Layer (`FunctionStorage`)

**Currently Implemented**: In-memory storage with full CRUD operations
**Planned**: Persistent storage backends (PostgreSQL, etc.)

Persistent storage for:
- **Function Definitions**: Schema, configuration, triggers
- **Execution Records**: Runtime data, logs, results, status tracking
- **Chain History**: Function execution graphs
- **Performance Metrics**: Timing, resource usage, success rates

Current storage operations:
```rust
// All implemented and working
async fn create_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition>;
async fn get_function(&self, id: &FunctionId) -> Result<Option<FunctionDefinition>>;
async fn update_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition>;
async fn delete_function(&self, id: &FunctionId) -> Result<bool>;
async fn list_functions(&self) -> Result<Vec<FunctionDefinition>>;
async fn create_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution>;
async fn update_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution>;
async fn get_execution(&self, id: &Uuid) -> Result<Option<FunctionExecution>>;
async fn list_executions(&self, function_id: &FunctionId) -> Result<Vec<FunctionExecution>>;
```

### 6. Rules Engine Integration (Planned)

Function outputs will become available in the rules engine:

```rust
// Function output becomes available in token metadata under "function_outputs"
// Structure: token.metadata.function_outputs.{function_id}.{execution_id}

// Rule that checks function output
Rule::field_equals(
    "document_has_high_confidence",
    "function_outputs.document_processor.confidence_score", 
    json!("> 0.8")
)

// Rule that checks if specific function completed
Rule::field_exists(
    "document_processed",
    "function_outputs.document_processor"
)
```

## Container Configuration

### Basic Container Setup

```rust
ContainerConfig::new("python:3.11-slim")
    .with_working_dir("/app")
    .with_env_var("PYTHONPATH", "/app")
    .with_setup_command(vec!["pip", "install", "-r", "requirements.txt"])
    .with_exec(vec!["python", "main.py"])
    .with_mount(ContainerMount {
        source: "/host/data",
        target: "/app/data",
        readonly: true,
    })
    .with_resources(ResourceLimits {
        memory_mb: Some(1024),
        cpu_cores: Some(1.0),
        timeout_seconds: Some(600),
    })
```

### Current Docker Execution Features

**✅ Implemented**:
- Image pulling and container creation
- Environment variable injection (including execution context)
- Working directory configuration
- Resource limits (memory, CPU, timeout)
- Volume mounts with read-only support
- Setup command execution before main command
- Real-time stdout/stderr capture
- Automatic container cleanup
- Exit code capture and reporting

**Execution Context Variables** (automatically injected):
```bash
TRIGGER_EVENT={"event_type":"TokenCreated","data":{"file":"test.pdf"}}
EXECUTION_ID=550e8400-e29b-41d4-a716-446655440000
FUNCTION_ID=document_processor
INPUT_DATA={"file_url":"https://example.com/file.pdf","type":"pdf"}
```

### Environment Variables and Secrets

```rust
ContainerConfig::new("myapp:latest")
    // Regular environment variables (✅ Implemented)
    .with_env_var("LOG_LEVEL", "info")
    .with_env_var("WORKER_COUNT", "4")
    
    // Secret variables (📋 Planned - secure resolution at runtime)
    .with_secret_var("DATABASE_URL", "postgres_connection")
    .with_secret_var("API_KEY", "external_api_key")
    
    // Function execution context (✅ Implemented - automatically injected)
    // TRIGGER_EVENT: JSON of the event that triggered this function
    // EXECUTION_ID: Unique ID for this execution
    // FUNCTION_ID: ID of the function being executed
    // INPUT_DATA: Processed input data for the function
```

## Function Development Patterns

### 1. Simple Data Processor (✅ Working)

```typescript
// Input: { "text": "Hello world", "language": "en" }
// Output: { "word_count": 2, "sentiment": "neutral", "language": "en" }

interface Input {
  text: string;
  language: string;
}

interface Output {
  word_count: number;
  sentiment: "positive" | "negative" | "neutral";
  language: string;
}

async function processText(input: Input): Promise<Output> {
  const words = input.text.split(/\s+/).length;
  const sentiment = await analyzeSentiment(input.text);
  
  return {
    word_count: words,
    sentiment: sentiment,
    language: input.language
  };
}

// Output the result as JSON on the last line for parsing
console.log(JSON.stringify(result));
```

### 2. API Integration Function (✅ Working)

```python
# Input: { "user_id": "123", "action": "login" }
# Output: { "event_logged": true, "timestamp": "2024-01-01T12:00:00Z" }

import json
import os
import requests
from datetime import datetime

def main():
    # Get function execution context (automatically available)
    trigger_event = json.loads(os.environ['TRIGGER_EVENT'])
    execution_id = os.environ['EXECUTION_ID']
    input_data = json.loads(os.environ['INPUT_DATA'])
    
    # Call external API
    response = requests.post('https://analytics.example.com/events', {
        'user_id': input_data['user_id'],
        'action': input_data['action'],
        'timestamp': datetime.utcnow().isoformat(),
        'execution_id': execution_id
    })
    
    # Return structured output (parsed by engine)
    output = {
        'event_logged': response.status_code == 200,
        'timestamp': datetime.utcnow().isoformat(),
        'response_code': response.status_code
    }
    
    # Output as JSON on last line for parsing
    print(json.dumps(output))

if __name__ == "__main__":
    main()
```

### 3. File Processing Chain (🚧 Chaining logic exists but disabled)

```rust
// Function 1: Extract data from uploaded file
FunctionDefinition {
    id: "file_extractor",
    triggers: [EventTrigger::on_token_created("file_uploaded", Some("uploaded"))],
    input_schema: schema!({
        "file_url": {"type": "string", "format": "uri"},
        "file_type": {"type": "string"}
    }),
    output_schema: schema!({
        "extracted_data": {"type": "object"},
        "record_count": {"type": "integer"},
        "errors": {"type": "array"}
    }),
    chains: [
        FunctionChain {
            target_function: "data_validator",
            condition: ChainCondition::ConditionalRule(
                Rule::field_greater_than("has_records", "record_count", 0.0)
            ),
            input_mapping: InputMapping::FieldMapping({
                "extracted_data": "data_to_validate"
            })
        }
    ]
}
```

## Error Handling and Monitoring

### Execution States (✅ Implemented)

- `Pending`: Function queued for execution
- `Starting`: Container being created  
- `Running`: Function executing
- `Completed`: Successful execution
- `Failed`: Execution failed
- `Timeout`: Execution exceeded time limit
- `Cancelled`: Execution was cancelled

### Current Error Handling Features

**✅ Working**:
- Docker execution errors with detailed error messages
- Container exit code capture and reporting
- Real-time stderr capture for debugging
- Input validation error reporting
- Output parsing error handling
- Execution status tracking with timestamps

**Example Error Output**:
```
❌ Docker container failed (exit code: 1)
⚠️  STDERR: ModuleNotFoundError: No module named 'requests'
⚠️  STDERR: Process completed with exit code 1
```

### Retry Mechanisms (🚧 Logic implemented, scheduling needs work)

```rust
RetryConfig {
    max_attempts: 3,
    retry_delay: Duration::seconds(30),
    backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
    retry_conditions: vec![
        RetryCondition::ExitCode(vec![1, 2]), // Retry on specific exit codes
        RetryCondition::Timeout,              // Retry on timeout
        RetryCondition::ContainerFailure,     // Retry on container failures
    ]
}
```

### Monitoring and Observability (✅ Basic implementation)

- **Execution Logs**: Captured stdout/stderr from containers with real-time display
- **Metrics**: Execution time, exit codes, success/failure rates
- **Status Tracking**: Complete execution lifecycle monitoring
- **Resource Usage**: Container resource limit enforcement

## Current Working Demo

The function runner can be tested with the working demo:

```bash
cargo run --example function_demo
```

**Current Demo Output**:
```
🐳 Executing function 550e8400-e29b-41d4-a716-446655440000 with Docker...
🔧 Running Docker command: docker run --name circuit-breaker-550e8400-e29b-41d4-a716-446655440000 --rm -e NODE_ENV=production -e TRIGGER_EVENT=TokenCreated... node:18-alpine echo {"processed": true, "timestamp": "2024-01-01T12:00:00Z"}
📦 Starting Docker container...
📄 STDOUT: {"processed": true, "timestamp": "2024-01-01T12:00:00Z"}
✅ Docker container completed successfully (exit code: 0)
✅ Function 550e8400-e29b-41d4-a716-446655440000 completed successfully
```

## GraphQL API (📋 Planned)

### Queries

```graphql
# Get all functions
query {
  functions {
    id
    name
    description
    enabled
    triggers {
      eventType
      conditions
    }
    inputSchema
    outputSchema
  }
}

# Get function executions
query {
  functionExecutions(functionId: "document_processor") {
    id
    status
    startedAt
    completedAt
    exitCode
    output
    error
  }
}

# Get execution chain for a token
query {
  tokenExecutionChain(tokenId: "token-123") {
    executions {
      functionId
      status
      input
      output
      triggeredBy
      chains {
        targetFunction
        status
      }
    }
  }
}
```

### Mutations

```graphql
# Create a function
mutation {
  createFunction(input: {
    name: "Image Processor"
    description: "Resize and optimize images"
    container: {
      image: "image-processor:latest"
      execCommand: ["python", "process.py"]
      resources: {
        memoryMb: 1024
        cpuCores: 1.0
        timeoutSeconds: 300
      }
    }
    inputSchema: "{\"type\": \"object\", \"properties\": {\"image_url\": {\"type\": \"string\"}}}"
    outputSchema: "{\"type\": \"object\", \"properties\": {\"processed_url\": {\"type\": \"string\"}}}"
  }) {
    id
    name
  }
}

# Add trigger to function
mutation {
  addFunctionTrigger(input: {
    functionId: "image_processor"
    eventType: "{\"type\": \"TokenCreated\", \"place\": \"image_uploaded\"}"
    workflowId: "image_workflow"
  }) {
    id
    triggers {
      eventType
    }
  }
}

# Trigger custom event
mutation {
  triggerCustomEvent(input: {
    eventName: "user_action"
    workflowId: "user_workflow"
    data: {action: "purchase", amount: 99.99}
  }) {
    executionIds
  }
}
```

## Integration with Rules Engine (📋 Planned)

Function outputs will be automatically available in the rules engine:

```rust
// After a function completes, its output will be stored in token metadata:
// token.metadata.function_outputs.{function_id}.{execution_id}

// Rules can check function results
Rule::field_exists("document_processed", "function_outputs.document_processor")
Rule::field_equals("high_confidence", "function_outputs.ai_classifier.confidence", json!("> 0.9"))
Rule::field_greater_than("good_score", "function_outputs.quality_checker.score", 8.0)

// Rules can check if function chain completed
Rule::and("processing_complete", "All processing steps completed", vec![
    Rule::field_exists("extracted", "function_outputs.extractor"),
    Rule::field_exists("validated", "function_outputs.validator"),
    Rule::field_exists("imported", "function_outputs.importer"),
])

// Transitions can depend on function results
TransitionDefinition::with_rules(
    "proceed_to_review",
    vec!["processing"],
    "review",
    vec![
        Rule::field_equals("auto_approved", "function_outputs.content_checker.approved", json!(true))
    ]
)
```

## Security Considerations

### Container Isolation (✅ Implemented)

- Functions run in isolated Docker containers
- Resource limits prevent resource exhaustion (memory, CPU, timeout)
- Automatic container cleanup after execution
- No host system access by default

### Secret Management (📋 Planned)

- Secrets resolved at runtime, never stored in function definitions
- Environment variable injection for secure credential access
- Integration with external secret management systems
- Audit trail of secret access

### Input Validation (🚧 Framework ready)

- JSON Schema validation framework in place
- Size limits for input data (configurable)
- Rate limiting for function execution (planned)
- Sanitization of user-provided data (planned)

## Performance Considerations

### Resource Management (✅ Implemented)

- Container resource limits (memory, CPU, timeout)
- Automatic container cleanup to prevent resource leaks
- Real-time resource monitoring during execution
- Configurable execution timeouts

### Optimization Strategies (📋 Planned)

- Container image optimization for faster startup
- Pre-warming containers for latency-sensitive functions
- Batch processing for multiple events
- Caching of function outputs where appropriate
- Container reuse strategies for frequently-called functions

## Implementation Roadmap

### Phase 1: Core Infrastructure ✅ COMPLETED
- [x] Function definition models
- [x] Event bus system  
- [x] Docker execution engine with real-time monitoring
- [x] Storage layer for functions and executions (in-memory)
- [x] Container lifecycle management
- [x] Error handling and status tracking

### Phase 2: Schema and Validation 🚧 IN PROGRESS
- [x] JSON Schema framework integration
- [ ] Input/output validation implementation
- [x] Error handling and retry mechanisms (logic ready)
- [ ] GraphQL API for function management

### Phase 3: Function Chaining 🚧 PARTIALLY DONE
- [x] Chain condition evaluation logic
- [x] Input mapping and transformation framework
- [ ] Chain execution tracking (fix lifetime issues)
- [ ] Rules engine integration for function outputs

### Phase 4: Advanced Features 📋 PLANNED
- [ ] Container optimization and reuse
- [ ] Secret management integration
- [ ] Enhanced monitoring and observability
- [ ] Performance optimization

### Phase 5: Developer Experience 📋 PLANNED
- [ ] Function development templates
- [ ] Local testing framework
- [ ] CLI tools for function deployment
- [ ] Enhanced documentation and examples

## Current Limitations

1. **Function Chaining**: Temporarily disabled due to Rust lifetime issues in async contexts
2. **Persistent Storage**: Only in-memory storage implemented (PostgreSQL integration planned)
3. **Secret Management**: Environment variables only, no secure secret resolution
4. **Schema Validation**: Framework ready but not fully integrated
5. **GraphQL API**: Not yet implemented
6. **Container Reuse**: Containers are created fresh for each execution

## Next Steps

1. **Fix Function Chaining**: Resolve lifetime issues with async execution contexts
2. **Persistent Storage**: Implement PostgreSQL-based storage backend
3. **Schema Validation**: Complete JSON Schema integration for input/output validation
4. **Secret Management**: Implement secure credential resolution system
5. **GraphQL API**: Build management and monitoring API

This function runner system provides a solid foundation for serverless-style computation within Circuit Breaker workflows, with core Docker execution working reliably and a clear path for advanced features. 