# TypeScript Streaming Implementation

## Overview

This document describes the TypeScript implementation of token-by-token streaming for the Circuit Breaker LLM Router, providing equivalent functionality to the Rust implementation with JavaScript/TypeScript-specific optimizations and patterns.

## Architecture

### Core Components

#### 1. Streaming Manager (`StreamingManagerImpl`)

Manages streaming sessions and provides lifecycle management:

```typescript
interface StreamingManager {
  config: StreamingConfig;
  activeSessions: Map<string, StreamingSession>;
  activeStreams: Map<string, any>;
}
```

**Key Features:**
- Session creation and cleanup
- Concurrent stream limiting
- Automatic session expiration
- Resource leak prevention

#### 2. LLM Router (`LLMRouter`)

Provides unified interface for streaming across all providers:

```typescript
async *streamChatCompletion(request: LLMRequest): AsyncGenerator<StreamingChunk, void, unknown>
```

**Features:**
- Async generator pattern for streaming
- Provider-agnostic interface
- Automatic error handling and recovery
- Server-Sent Events parsing

#### 3. SSE Parser (`SSEParser`)

Handles Server-Sent Events parsing with provider-specific support:

```typescript
class SSEParser {
  parseChunk(chunk: string): Array<{ eventType?: string; data: string; id?: string }>;
}
```

### Provider-Specific Implementations

#### OpenAI Streaming
```typescript
class OpenAISSEParser {
  static parseEvent(event: { data: string }): StreamingChunk | null;
}
```
- Standard SSE format with `data: {json}` events
- Delta content extraction
- Finish reason detection

#### Anthropic Streaming
```typescript
class AnthropicSSEParser {
  static parseEvent(event: { data: string }, requestId: string, model: string): StreamingChunk | null;
}
```
- Event-based SSE with typed events
- `content_block_delta` parsing
- Message lifecycle management

#### Google Streaming
```typescript
class GoogleSSEParser {
  static parseEvent(event: { data: string }, requestId: string, model: string): StreamingChunk | null;
}
```
- Candidate-based response format
- Multi-part content extraction
- Stream completion detection

## Key TypeScript Features

### Async Generators

The implementation uses TypeScript's async generator pattern for streaming:

```typescript
async *streamChatCompletion(request: LLMRequest): AsyncGenerator<StreamingChunk, void, unknown> {
  // Server-Sent Events parsing
  const response = await fetch(url, { ... });
  const reader = response.body.getReader();
  
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      
      // Parse and yield chunks
      for (const chunk of parseSSE(value)) {
        yield chunk;
      }
    }
  } finally {
    reader.releaseLock();
  }
}
```

### Type Safety

Full TypeScript type definitions for all streaming components:

```typescript
interface StreamingChunk {
  id: string;
  object: string;
  choices: StreamingChoice[];
  created: number;
  model: string;
  provider: string;
}

interface StreamingChoice {
  index: number;
  delta: ChatMessage;
  finishReason?: string;
}
```

### Error Handling

Comprehensive error handling with TypeScript's type system:

```typescript
try {
  for await (const chunk of router.streamChatCompletion(request)) {
    // Process chunk
  }
} catch (error) {
  if (error instanceof NetworkError) {
    // Handle network issues
  } else if (error instanceof ParseError) {
    // Handle parsing issues
  }
}
```

## Usage Examples

### Basic Streaming

```typescript
import { LLMRouter, LLMRequest } from './streaming_architecture_demo';

const router = await LLMRouter.new();
const request: LLMRequest = {
  id: uuidv4(),
  model: "claude-sonnet-4-20250514",
  messages: [{ role: "user", content: "Hello!" }],
  stream: true,
  metadata: {},
};

// Stream tokens as they arrive
for await (const chunk of router.streamChatCompletion(request)) {
  if (chunk.choices[0]?.delta?.content) {
    process.stdout.write(chunk.choices[0].delta.content);
  }
}
```

### Advanced Session Management

```typescript
const streamingManager = new StreamingManagerImpl({
  maxConcurrentStreams: 100,
  defaultBufferSize: 50,
  sessionTimeoutMs: 300000,
  maxChunkSize: 8192,
  enableFlowControl: true,
});

const sessionId = await streamingManager.createSession(
  "ServerSentEvents",
  "user-123",
  "project-456"
);

// Use session...

await streamingManager.closeSession(sessionId);
```

### Provider-Specific Parsing

```typescript
// Parse Anthropic events
const anthropicChunk = AnthropicSSEParser.parseEvent(
  { data: '{"type":"content_block_delta","delta":{"text":"Hello"}}' },
  "request-123",
  "claude-3-sonnet"
);

// Parse OpenAI events
const openaiChunk = OpenAISSEParser.parseEvent({
  data: '{"choices":[{"delta":{"content":"Hello"}}]}'
});
```

## Running the Demo

### Installation

```bash
cd circuit-breaker/examples/typescript
npm install
```

### Basic Demo

```bash
npm run demo:streaming
```

### With API Key

```bash
export ANTHROPIC_API_KEY="your-key-here"
npm run demo:streaming
```

## Performance Characteristics

### Memory Efficiency

- **Streaming Buffers**: Bounded buffers prevent memory leaks
- **Async Generators**: Lazy evaluation reduces memory usage
- **Session Cleanup**: Automatic cleanup prevents resource accumulation

### Latency Improvements

- **First Token Time**: ~200-500ms vs 2-10 seconds (full response)
- **Progressive Rendering**: Immediate user feedback
- **Network Efficiency**: Stream processing reduces perceived latency

### Concurrency

- **Multiple Streams**: Support for concurrent streaming sessions
- **Resource Limits**: Configurable limits prevent resource exhaustion
- **Session Management**: Proper lifecycle management

## Error Handling Strategies

### Network Errors

```typescript
try {
  const response = await fetch(url, options);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
} catch (error) {
  if (error.code === 'ECONNREFUSED') {
    // Fallback to alternative provider
  }
}
```

### Parse Errors

```typescript
try {
  const chunk = JSON.parse(data);
  yield chunk;
} catch (parseError) {
  console.warn('Skipping malformed chunk:', data);
  continue; // Skip and continue streaming
}
```

### Stream Interruption

```typescript
const reader = response.body.getReader();
try {
  // Stream processing
} finally {
  reader.releaseLock(); // Always cleanup
}
```

## Integration with Node.js

### Dependencies

```json
{
  "dependencies": {
    "node-fetch": "^3.3.2",
    "ws": "^8.14.2",
    "uuid": "^9.0.1"
  }
}
```

### ES Modules

Full ES module support with proper TypeScript compilation:

```typescript
// streaming_architecture_demo.ts
import fetch from "node-fetch";
import { v4 as uuidv4 } from "uuid";

export class LLMRouter {
  // Implementation
}
```

### Environment Configuration

```typescript
import { config } from "dotenv";
config(); // Load .env file

const apiKey = process.env.ANTHROPIC_API_KEY;
```

## Browser Compatibility

### Fetch Streams

Compatible with modern browsers supporting Fetch API streams:

```typescript
// Browser-compatible streaming
const response = await fetch(url);
const reader = response.body?.getReader();
```

### WebSocket Alternative

For browsers without SSE support:

```typescript
const ws = new WebSocket('ws://localhost:4000/ws');
ws.onmessage = (event) => {
  const chunk = JSON.parse(event.data);
  // Process streaming chunk
};
```

## Testing

### Unit Tests

```typescript
import { SSEParser } from './streaming_architecture_demo';

test('SSE parser handles multiple events', () => {
  const parser = new SSEParser();
  const events = parser.parseChunk('data: event1\n\ndata: event2\n\n');
  expect(events).toHaveLength(2);
});
```

### Integration Tests

```typescript
test('streaming with mock server', async () => {
  const router = await LLMRouter.new();
  const chunks = [];
  
  for await (const chunk of router.streamChatCompletion(testRequest)) {
    chunks.push(chunk);
  }
  
  expect(chunks.length).toBeGreaterThan(0);
});
```

## Monitoring and Observability

### Streaming Metrics

```typescript
class StreamingMetrics {
  private chunkCount = 0;
  private totalLatency = 0;
  
  recordChunk(latencyMs: number) {
    this.chunkCount++;
    this.totalLatency += latencyMs;
  }
  
  getAverageLatency(): number {
    return this.totalLatency / this.chunkCount;
  }
}
```

### Debug Logging

```typescript
console.log(`🔍 ${provider} API Streaming Request:`);
console.log(`   URL: ${url}`);
console.log(`   Model: ${request.model}`);
```

## Future Enhancements

### Planned Features

1. **WebRTC Streaming** - Ultra-low latency streaming
2. **Compression Support** - Gzip/deflate for reduced bandwidth
3. **Retry Logic** - Automatic reconnection with exponential backoff
4. **Metrics Collection** - Comprehensive streaming analytics
5. **Cache Integration** - Response caching for improved performance

### Performance Optimizations

1. **Connection Pooling** - HTTP/2 multiplexing
2. **Buffering Strategies** - Adaptive buffer sizing
3. **Parallel Streaming** - Multiple provider streams
4. **Edge Computing** - CDN-based streaming acceleration

## Security Considerations

### API Key Management

```typescript
// Never log API keys
const maskedKey = apiKey.slice(0, 4) + '***';
console.log(`Using API key: ${maskedKey}`);
```

### Input Validation

```typescript
function validateRequest(request: LLMRequest): void {
  if (!request.model || typeof request.model !== 'string') {
    throw new Error('Invalid model specification');
  }
  // Additional validation...
}
```

### Rate Limiting

```typescript
class RateLimiter {
  private requests = new Map<string, number[]>();
  
  checkLimit(userId: string, maxPerMinute: number): boolean {
    const now = Date.now();
    const userRequests = this.requests.get(userId) || [];
    const recentRequests = userRequests.filter(time => now - time < 60000);
    
    return recentRequests.length < maxPerMinute;
  }
}
```

## Conclusion

The TypeScript streaming implementation provides:

✅ **Production-Ready Streaming** - Full SSE parsing and token-by-token delivery  
✅ **Type Safety** - Complete TypeScript type definitions  
✅ **Multi-Provider Support** - OpenAI, Anthropic, Google compatibility  
✅ **Performance** - Significant latency and memory improvements  
✅ **Developer Experience** - Modern async/await patterns  
✅ **Browser Compatible** - Works in both Node.js and browser environments  

This implementation enables real-time LLM applications with professional-grade streaming capabilities in the JavaScript/TypeScript ecosystem.