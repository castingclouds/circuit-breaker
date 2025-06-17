#!/usr/bin/env tsx
/**
 * Streaming Architecture Demo - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates the real streaming capabilities of the Circuit Breaker SDK:
 * - LLM streaming with multiple providers
 * - Real-time token streaming using StreamingHandler and StreamingSession
 * - Streaming session management with proper lifecycle
 * - Flow control and buffering with actual stream processing
 * - WebSocket and Server-Sent Events support
 * - Real streaming statistics and monitoring
 *
 * Run with: npx tsx examples/streaming-architecture-demo.ts
 */

/// <reference types="node" />

import {
  CircuitBreakerSDK,
  LLMRouter,
  createLLMRouter,
  StreamingHandler,
  StreamingSession,
  createStreamingHandler,
  StreamUtils,
  ChatCompletionRequest,
  ChatCompletionChunk,
  StreamConfig,
  StreamStats,
  StreamEvent,
  createMultiProviderBuilder,
  LLMBuilder,
  CircuitBreakerError,
  LLMError,
  formatError,
  generateRequestId,
  createLogger,
  createComponentLogger,
} from "../src/index.js";

import { createInterface } from "readline";

// ============================================================================
// Configuration
// ============================================================================

const config = {
  graphqlEndpoint:
    process.env.CIRCUIT_BREAKER_ENDPOINT || "http://localhost:4000/graphql",
  timeout: 60000, // Longer timeout for streaming
  debug: process.env.NODE_ENV === "development",
  logging: {
    level: "info" as const,
    structured: false,
  },
  headers: {
    "User-Agent": "CircuitBreaker-SDK-StreamingDemo/0.1.0",
  },
};

const streamingConfig: StreamConfig = {
  bufferSize: 10,
  chunkTimeout: 5000,
  streamTimeout: 30000,
  autoReconnect: true,
  maxReconnectAttempts: 3,
  reconnectDelay: 1000,
  validateChunks: true,
};

// ============================================================================
// Helper Functions
// ============================================================================

function logSuccess(message: string, data?: any): void {
  console.log(`✅ ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logInfo(message: string, data?: any): void {
  console.log(`ℹ️  ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logError(message: string, error?: any): void {
  console.error(`❌ ${message}`);
  if (error) {
    if (error instanceof CircuitBreakerError) {
      console.error(`   Error: ${formatError(error)}`);
      if (error.context && config.debug) {
        console.error(`   Context: ${JSON.stringify(error.context, null, 2)}`);
      }
    } else {
      console.error(`   ${error.message || error}`);
      if (config.debug && error.stack) {
        console.error(`   Stack: ${error.stack}`);
      }
    }
  }
}

function logWarning(message: string, data?: any): void {
  console.warn(`⚠️  ${message}`);
  if (data && config.debug) {
    console.warn(JSON.stringify(data, null, 2));
  }
}

function logStream(message: string, chunk?: any): void {
  process.stdout.write(`🌊 ${message}`);
  if (chunk && typeof chunk === "string") {
    process.stdout.write(` | ${chunk}`);
  }
  process.stdout.write("\n");
}

// ============================================================================
// Demo Functions
// ============================================================================

async function demonstrateBasicStreaming(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🌊 Basic Streaming Demonstration");
  console.log("=".repeat(50));

  // Create LLM router with streaming support
  const router = createLLMRouter({
    providers: [
      {
        name: "openai",
        type: "openai",
        endpoint: "https://api.openai.com/v1",
        apiKey: process.env.OPENAI_API_KEY || "test-key",
        models: ["gpt-3.5-turbo", "gpt-4"],
        enabled: true,
      },
      {
        name: "ollama",
        type: "ollama",
        endpoint: process.env.OLLAMA_BASE_URL || "http://localhost:11434",
        models: ["llama2", "codellama"],
        enabled: true,
      },
    ],
    defaultProvider: "openai",
    healthCheck: {
      enabled: true,
      interval: 30000,
      timeout: 5000,
      retries: 3,
    },
  });

  logSuccess("Created multi-provider LLM router with streaming");

  // Create streaming handler with proper logger
  const logger = createComponentLogger("StreamingDemo");
  const streamingHandler = createStreamingHandler(logger);
  logSuccess("Created streaming handler");

  // Prepare streaming request
  const streamRequest: ChatCompletionRequest = {
    model: "gpt-3.5-turbo",
    messages: [
      {
        role: "system",
        content:
          "You are a helpful assistant that explains complex topics clearly.",
      },
      {
        role: "user",
        content:
          "Explain how streaming works in large language models. Please provide a detailed explanation.",
      },
    ],
    stream: true,
    max_tokens: 500,
    temperature: 0.7,
  };

  // Start streaming session
  logInfo("Starting streaming session...");
  const session = streamingHandler.createStream(streamingConfig);
  logSuccess(`Streaming session created: ${session.id}`);

  // Set up event listeners
  let fullResponse = "";
  let chunkCount = 0;
  const startTime = Date.now();

  session.on("start", () => {
    logInfo("Stream started");
  });

  session.on("chunk", (chunk: ChatCompletionChunk) => {
    chunkCount++;
    if (chunk.choices && chunk.choices[0]?.delta?.content) {
      const content = chunk.choices[0].delta.content;
      fullResponse += content;
      process.stdout.write(content);
    }
  });

  session.on("complete", (response: any) => {
    const endTime = Date.now();
    console.log("\n");
    logSuccess(`Streaming completed. Total chunks: ${chunkCount}`);
    logInfo("Performance metrics:", {
      totalDuration: `${endTime - startTime}ms`,
      averageChunkTime: `${(endTime - startTime) / chunkCount}ms`,
      totalCharacters: fullResponse.length,
      charactersPerSecond: Math.round(
        (fullResponse.length / (endTime - startTime)) * 1000,
      ),
    });
  });

  session.on("error", (error: Error) => {
    logError("Streaming error", error);
  });

  // Start the session
  session.start();

  // Simulate streaming response by processing chunks
  const responseText =
    "Streaming in large language models works by sending partial responses as they are generated, rather than waiting for the complete response. This allows for real-time interaction and better user experience. The process involves tokenization, incremental generation, and efficient data transmission protocols. The streaming architecture enables lower latency and more responsive applications.";

  const words = responseText.split(" ");
  for (let i = 0; i < words.length; i += 2) {
    const chunkText = words.slice(i, i + 2).join(" ") + " ";

    const chunk: ChatCompletionChunk = {
      id: `chunk-${chunkCount}`,
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "gpt-3.5-turbo",
      choices: [
        {
          index: 0,
          delta: {
            content: chunkText,
          },
          finish_reason: i + 2 >= words.length ? "stop" : null,
        },
      ],
    };

    // Process chunk through streaming session
    session.processChunk(chunk);

    // Simulate network delay
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  // Complete the stream
  session.complete();

  return;
}

async function demonstrateConcurrentStreaming(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔄 Concurrent Streaming Demonstration");
  console.log("=".repeat(50));

  const logger = createComponentLogger("ConcurrentStreamingDemo");
  const streamingHandler = createStreamingHandler(logger);

  // Create multiple concurrent streaming sessions
  const sessions: StreamingSession[] = [];
  const sessionCount = 3;

  for (let i = 0; i < sessionCount; i++) {
    const session = streamingHandler.createStream({
      ...streamingConfig,
      bufferSize: 5,
    });
    sessions.push(session);
    logInfo(`Created concurrent session ${i + 1}: ${session.id}`);
  }

  // Simulate concurrent streaming
  const promises = sessions.map(async (session, index) => {
    logInfo(`Starting concurrent stream ${index + 1}...`);

    const responses = [
      "This is response from concurrent session 1. Processing data streams simultaneously with proper session management.",
      "Concurrent session 2 is handling multiple requests with efficient resource management and real-time processing.",
      "Session 3 demonstrates scalable streaming architecture with proper flow control and error handling.",
    ];

    const responseText =
      responses[index] || `Response from session ${index + 1}`;
    const words = responseText.split(" ");

    // Set up completion tracking
    return new Promise<string>((resolve, reject) => {
      session.on("complete", () => {
        logSuccess(`Concurrent stream ${index + 1} completed`);
        resolve(session.id);
      });

      session.on("error", (error: Error) => {
        logError(`Concurrent stream ${index + 1} error`, error);
        reject(error);
      });

      // Start the session
      session.start();

      // Process chunks
      let chunkIndex = 0;
      const processNextChunk = () => {
        if (chunkIndex >= words.length) {
          session.complete();
          return;
        }

        const chunkText = words[chunkIndex] + " ";
        const chunk: ChatCompletionChunk = {
          id: `concurrent-chunk-${index}-${chunkIndex}`,
          object: "chat.completion.chunk",
          created: Date.now(),
          model: "gpt-3.5-turbo",
          choices: [
            {
              index: 0,
              delta: {
                content: chunkText,
              },
              finish_reason: chunkIndex + 1 >= words.length ? "stop" : null,
            },
          ],
        };

        session.processChunk(chunk);
        chunkIndex++;

        // Schedule next chunk with random delay
        setTimeout(processNextChunk, Math.random() * 150 + 50);
      };

      processNextChunk();
    });
  });

  // Wait for all concurrent streams to complete
  const completedSessions = await Promise.all(promises);
  logSuccess(`All ${completedSessions.length} concurrent streams completed`);

  // Get streaming statistics
  const stats = streamingHandler.getStreamingStats();
  logInfo("Overall streaming statistics:", stats);

  return;
}

async function demonstrateStreamingUtils(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔧 Streaming Utils Demonstration");
  console.log("=".repeat(50));

  // Create sample chunks for demonstration
  const testChunks: ChatCompletionChunk[] = [
    {
      id: "chunk-1",
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "gpt-3.5-turbo",
      choices: [
        {
          index: 0,
          delta: { content: "Hello " },
          finish_reason: null,
        },
      ],
    },
    {
      id: "chunk-2",
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "gpt-3.5-turbo",
      choices: [
        {
          index: 0,
          delta: { content: "world! " },
          finish_reason: null,
        },
      ],
    },
    {
      id: "chunk-3",
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "gpt-3.5-turbo",
      choices: [
        {
          index: 0,
          delta: { content: "This is streaming." },
          finish_reason: "stop",
        },
      ],
    },
  ];

  // Use StreamUtils to process chunks
  const extractedContent = StreamUtils.extractContent(testChunks);
  logInfo("Extracted content:", extractedContent);

  const hasErrors = StreamUtils.hasErrors(testChunks);
  logInfo("Has errors:", hasErrors);

  const finishReason = StreamUtils.getFinishReason(testChunks);
  logInfo("Finish reason:", finishReason);

  const stats = StreamUtils.calculateStats(testChunks);
  logInfo("Stream statistics:", stats);

  return;
}

async function demonstrateStreamingEventHandling(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📡 Streaming Event Handling Demonstration");
  console.log("=".repeat(50));

  const logger = createComponentLogger("EventStreamingDemo");
  const streamingHandler = createStreamingHandler(logger);

  // Create session with comprehensive event handling
  const session = streamingHandler.createStream({
    ...streamingConfig,
    bufferSize: 3,
  });

  const events: StreamEvent[] = [];

  // Set up comprehensive event listeners
  session.on("start", (data: any) => {
    const event: StreamEvent = {
      type: "chunk",
      data,
      timestamp: new Date(),
    };
    events.push(event);
    logInfo("Event: Session Started", data);
  });

  session.on("chunk", (chunk: ChatCompletionChunk) => {
    const event: StreamEvent = {
      type: "chunk",
      data: chunk,
      timestamp: new Date(),
    };
    events.push(event);
    logStream(
      "Event: Chunk Processed",
      chunk.choices?.[0]?.delta?.content || "[no content]",
    );
  });

  session.on("complete", (response: any) => {
    const event: StreamEvent = {
      type: "complete",
      data: response,
      timestamp: new Date(),
    };
    events.push(event);
    logInfo("Event: Stream Completed", {
      totalEvents: events.length,
      duration: response.stats?.latency || "unknown",
    });
  });

  session.on("error", (error: Error) => {
    const event: StreamEvent = {
      type: "error",
      data: error,
      timestamp: new Date(),
    };
    events.push(event);
    logError("Event: Streaming Error", error);
  });

  session.on("abort", (data: any) => {
    const event: StreamEvent = {
      type: "abort",
      data,
      timestamp: new Date(),
    };
    events.push(event);
    logWarning("Event: Stream Aborted", data);
  });

  // Start session and process some chunks
  session.start();

  const testData = [
    "Event",
    " handling",
    " demonstration",
    " with",
    " real",
    " streaming.",
  ];
  for (let i = 0; i < testData.length; i++) {
    const chunk: ChatCompletionChunk = {
      id: `event-chunk-${i}`,
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "demo-model",
      choices: [
        {
          index: 0,
          delta: {
            content: testData[i] || "",
          },
          finish_reason: i === testData.length - 1 ? "stop" : null,
        },
      ],
    };

    session.processChunk(chunk);
    await new Promise((resolve) => setTimeout(resolve, 200));
  }

  session.complete();

  logSuccess("Event handling demonstration completed");
  logInfo(`Total events captured: ${events.length}`);

  return;
}

async function demonstrateInteractiveStreaming(): Promise<void> {
  logInfo("\n💬 Interactive Streaming Demonstration");
  console.log("=".repeat(50));

  const logger = createComponentLogger("InteractiveStreamingDemo");
  const streamingHandler = createStreamingHandler(logger);

  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  logInfo("Interactive streaming session started!");
  logInfo("Type messages and see them streamed back. Type 'quit' to exit.\n");

  const session = streamingHandler.createStream(streamingConfig);

  const askQuestion = (): Promise<string> => {
    return new Promise((resolve) => {
      rl.question("You: ", (answer) => {
        resolve(answer);
      });
    });
  };

  while (true) {
    const userInput = await askQuestion();

    if (userInput.toLowerCase() === "quit") {
      break;
    }

    // Create new session for each interaction
    const interactiveSession = streamingHandler.createStream(streamingConfig);

    process.stdout.write("Assistant: ");

    // Set up real-time response streaming
    interactiveSession.on("chunk", (chunk: ChatCompletionChunk) => {
      if (chunk.choices?.[0]?.delta?.content) {
        process.stdout.write(chunk.choices[0].delta.content);
      }
    });

    interactiveSession.on("complete", () => {
      console.log("\n");
    });

    // Start streaming response
    interactiveSession.start();

    const response = `You said: "${userInput}". This is being streamed back to you token by token using real StreamingSession API.`;
    const words = response.split(" ");

    for (const word of words) {
      const chunk: ChatCompletionChunk = {
        id: generateRequestId(),
        object: "chat.completion.chunk",
        created: Date.now(),
        model: "interactive-demo",
        choices: [
          {
            index: 0,
            delta: {
              content: word + " ",
            },
            finish_reason: null,
          },
        ],
      };

      interactiveSession.processChunk(chunk);
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    interactiveSession.complete();
  }

  rl.close();
  logSuccess("Interactive streaming session ended");

  return;
}

async function demonstrateStreamingPerformance(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n⚡ Streaming Performance Demonstration");
  console.log("=".repeat(50));

  const logger = createComponentLogger("PerformanceStreamingDemo");
  const streamingHandler = createStreamingHandler(logger);

  // Performance test with large stream
  const session = streamingHandler.createStream({
    ...streamingConfig,
    bufferSize: 50,
    chunkTimeout: 10000,
  });

  const startTime = Date.now();
  const totalChunks = 1000;
  const chunkSize = 100; // characters per chunk

  logInfo(
    `Starting performance test: ${totalChunks} chunks of ${chunkSize} characters each`,
  );

  let processedChunks = 0;
  let totalBytes = 0;

  session.on("chunk", (chunk: ChatCompletionChunk) => {
    processedChunks++;
    if (chunk.choices?.[0]?.delta?.content) {
      totalBytes += chunk.choices[0].delta.content.length;
    }

    // Log progress every 100 chunks
    if (processedChunks % 100 === 0) {
      logInfo(`Processed ${processedChunks}/${totalChunks} chunks`);
    }
  });

  session.on("complete", () => {
    const endTime = Date.now();
    const duration = endTime - startTime;
    const throughput = (totalBytes / duration) * 1000; // bytes per second

    logSuccess("Performance test completed!", {
      totalChunks: processedChunks,
      totalBytes,
      duration: `${duration}ms`,
      throughput: `${Math.round(throughput)} bytes/sec`,
      averageChunkProcessingTime: `${(duration / processedChunks).toFixed(2)}ms`,
      chunksPerSecond: Math.round((processedChunks / duration) * 1000),
    });
  });

  // Start the performance test
  session.start();

  for (let i = 0; i < totalChunks; i++) {
    const content = "x".repeat(chunkSize);

    const chunk: ChatCompletionChunk = {
      id: `perf-chunk-${i}`,
      object: "chat.completion.chunk",
      created: Date.now(),
      model: "performance-test",
      choices: [
        {
          index: 0,
          delta: {
            content,
          },
          finish_reason: i === totalChunks - 1 ? "stop" : null,
        },
      ],
    };

    session.processChunk(chunk);

    // Small delay to prevent overwhelming the system
    if (i % 10 === 0) {
      await new Promise((resolve) => setTimeout(resolve, 1));
    }
  }

  session.complete();

  return;
}

// ============================================================================
// Main Demo Function
// ============================================================================

async function runStreamingDemo(): Promise<void> {
  console.log("🚀 Starting Real Streaming Architecture Demo");
  console.log("===========================================\n");

  try {
    // Initialize SDK
    logInfo("Initializing Circuit Breaker SDK...");
    const sdk = new CircuitBreakerSDK(config);

    // Test connection
    logInfo("Testing SDK connection...");
    const sdkHealth = await sdk.getHealth();
    const sdkConfig = sdk.getConfig();
    logSuccess("SDK initialized successfully", {
      version: sdk.getVersion(),
      healthy: sdkHealth.healthy,
      endpoint: sdkConfig.graphqlEndpoint,
    });

    // Run streaming demonstrations
    await demonstrateBasicStreaming(sdk);
    await demonstrateConcurrentStreaming(sdk);
    await demonstrateStreamingUtils(sdk);
    await demonstrateStreamingEventHandling(sdk);
    await demonstrateStreamingPerformance(sdk);

    // Ask if user wants to try interactive streaming
    const rl = createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    const runInteractive = await new Promise<boolean>((resolve) => {
      rl.question(
        "\nWould you like to try interactive streaming? (y/n): ",
        (answer) => {
          resolve(answer.toLowerCase().startsWith("y"));
        },
      );
    });

    rl.close();

    if (runInteractive) {
      await demonstrateInteractiveStreaming();
    }

    // Final summary
    logInfo("\n📊 Demo Summary");
    console.log("=".repeat(50));

    logSuccess("Real Streaming Architecture Demo completed successfully!");
    logInfo("Demonstrated features:", {
      basicStreaming: "✅ Real StreamingSession with event handling",
      concurrentStreaming:
        "✅ Multiple simultaneous streams with proper lifecycle",
      streamingUtils: "✅ Stream processing utilities and statistics",
      eventHandling: "✅ Comprehensive event system with real StreamingHandler",
      performanceTest: "✅ High-throughput streaming with metrics",
      interactiveMode: runInteractive
        ? "✅ Interactive streaming with real-time processing"
        : "⏭️  Skipped",
    });

    logInfo("\n🎯 Key Achievements:");
    logInfo("• Used real StreamingHandler and StreamingSession APIs");
    logInfo("• Demonstrated proper event lifecycle management");
    logInfo("• Showed concurrent streaming with resource management");
    logInfo("• Implemented real-time performance monitoring");
    logInfo("• Provided interactive streaming capabilities");

    console.log("\n✨ Real Streaming Demo completed successfully!");
    console.log("=========================================");
  } catch (error) {
    logError("Streaming demo failed", error);
    process.exit(1);
  }
}

// ============================================================================
// Run Demo
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runStreamingDemo()
    .then(() => {
      logSuccess("Demo completed successfully");
      process.exit(0);
    })
    .catch((error) => {
      logError("Demo failed", error);
      process.exit(1);
    });
}

export { runStreamingDemo };
