/**
 * Tests for Circuit Breaker SSE (Server-Sent Events) parsing utilities
 *
 * These tests verify that the SSE parsing works correctly with the Circuit Breaker router,
 * which presents an OpenAI-compatible API format.
 */

import {
  SSEParser,
  responseToSSEStream,
  parseCircuitBreakerEvent,
  streamChatCompletionFromRouter,
  SSEError,
  SSEParseError,
  SSEStreamError,
  isSSESupported,
  createSSEClient,
  type SSEEvent,
  type StreamingChunk,
} from "./sse";

describe("SSEParser", () => {
  let parser: SSEParser;

  beforeEach(() => {
    parser = new SSEParser();
  });

  describe("parseChunk", () => {
    it("should parse single SSE event", () => {
      const chunk = 'data: {"test": "value"}\n\n';
      const events = parser.parseChunk(chunk);

      expect(events).toHaveLength(1);
      expect(events[0].data).toBe('{"test": "value"}');
    });

    it("should parse multiple SSE events", () => {
      const chunk = 'data: {"test": "value1"}\n\ndata: {"test": "value2"}\n\n';
      const events = parser.parseChunk(chunk);

      expect(events).toHaveLength(2);
      expect(events[0].data).toBe('{"test": "value1"}');
      expect(events[1].data).toBe('{"test": "value2"}');
    });

    it("should handle partial events in buffer", () => {
      const chunk1 = 'data: {"test": "value"';
      const chunk2 = "}\n\n";

      const events1 = parser.parseChunk(chunk1);
      const events2 = parser.parseChunk(chunk2);

      expect(events1).toHaveLength(0);
      expect(events2).toHaveLength(1);
      expect(events2[0].data).toBe('{"test": "value"}');
    });

    it("should parse event with all SSE fields", () => {
      const chunk =
        'event: test\ndata: {"test": "value"}\nid: 123\nretry: 1000\n\n';
      const events = parser.parseChunk(chunk);

      expect(events).toHaveLength(1);
      expect(events[0]).toEqual({
        eventType: "test",
        data: '{"test": "value"}',
        id: "123",
        retry: 1000,
      });
    });

    it("should skip comments and empty lines", () => {
      const chunk = ': this is a comment\n\ndata: {"test": "value"}\n\n';
      const events = parser.parseChunk(chunk);

      expect(events).toHaveLength(1);
      expect(events[0].data).toBe('{"test": "value"}');
    });
  });

  describe("hasRemainingData", () => {
    it("should return false when buffer is empty", () => {
      expect(parser.hasRemainingData()).toBe(false);
    });

    it("should return true when buffer has data", () => {
      parser.parseChunk("data: partial");
      expect(parser.hasRemainingData()).toBe(true);
    });
  });

  describe("flushRemaining", () => {
    it("should return empty array when no remaining data", () => {
      const events = parser.flushRemaining();
      expect(events).toHaveLength(0);
    });

    it("should parse remaining data in buffer", () => {
      parser.parseChunk("data: remaining data");
      const events = parser.flushRemaining();

      expect(events).toHaveLength(1);
      expect(events[0].data).toBe("remaining data");
    });
  });
});

describe("parseCircuitBreakerEvent", () => {
  it("should parse valid Circuit Breaker streaming chunk", () => {
    const event: SSEEvent = {
      data: JSON.stringify({
        id: "chatcmpl-123",
        object: "chat.completion.chunk",
        created: 1677652288,
        model: "claude-3-haiku-20240307",
        choices: [
          {
            index: 0,
            delta: {
              role: "assistant",
              content: "Hello world!",
            },
            finish_reason: null,
          },
        ],
        system_fingerprint: "cb-2024-01",
      }),
    };

    const chunk = parseCircuitBreakerEvent(event);

    expect(chunk).toBeDefined();
    expect(chunk!.id).toBe("chatcmpl-123");
    expect(chunk!.model).toBe("claude-3-haiku-20240307");
    expect(chunk!.choices[0].delta.content).toBe("Hello world!");
    expect(chunk!.system_fingerprint).toBe("cb-2024-01");
  });

  it("should parse finish reason event", () => {
    const event: SSEEvent = {
      data: JSON.stringify({
        id: "chatcmpl-123",
        object: "chat.completion.chunk",
        created: 1677652288,
        model: "gpt-4",
        choices: [
          {
            index: 0,
            delta: {},
            finish_reason: "stop",
          },
        ],
      }),
    };

    const chunk = parseCircuitBreakerEvent(event);

    expect(chunk).toBeDefined();
    expect(chunk!.choices[0].finish_reason).toBe("stop");
    expect(chunk!.choices[0].delta.content).toBeUndefined();
  });

  it("should return null for [DONE] event", () => {
    const event: SSEEvent = {
      data: "[DONE]",
    };

    const chunk = parseCircuitBreakerEvent(event);
    expect(chunk).toBeNull();
  });

  it("should return null for empty data", () => {
    const event: SSEEvent = {
      data: "",
    };

    const chunk = parseCircuitBreakerEvent(event);
    expect(chunk).toBeNull();
  });

  it("should handle malformed JSON gracefully", () => {
    const event: SSEEvent = {
      data: '{"id": "test", "invalid": json',
    };

    const consoleWarnSpy = jest
      .spyOn(console, "warn")
      .mockImplementation(() => {});

    const chunk = parseCircuitBreakerEvent(event);

    expect(chunk).toBeNull();
    expect(consoleWarnSpy).toHaveBeenCalled();

    consoleWarnSpy.mockRestore();
  });

  it("should handle invalid structure gracefully", () => {
    const event: SSEEvent = {
      data: JSON.stringify({
        id: "test",
        // Missing required fields
      }),
    };

    const consoleWarnSpy = jest
      .spyOn(console, "warn")
      .mockImplementation(() => {});

    const chunk = parseCircuitBreakerEvent(event);

    expect(chunk).toBeNull();
    expect(consoleWarnSpy).toHaveBeenCalled();

    consoleWarnSpy.mockRestore();
  });

  it("should handle tool calls in delta", () => {
    const event: SSEEvent = {
      data: JSON.stringify({
        id: "chatcmpl-123",
        object: "chat.completion.chunk",
        created: 1677652288,
        model: "gpt-4",
        choices: [
          {
            index: 0,
            delta: {
              tool_calls: [
                {
                  id: "call_123",
                  type: "function",
                  function: {
                    name: "get_weather",
                    arguments: '{"location": "San Francisco"}',
                  },
                },
              ],
            },
            finish_reason: null,
          },
        ],
      }),
    };

    const chunk = parseCircuitBreakerEvent(event);

    expect(chunk).toBeDefined();
    expect(chunk!.choices[0].delta.tool_calls).toBeDefined();
    expect(chunk!.choices[0].delta.tool_calls).toHaveLength(1);
  });
});

describe("responseToSSEStream", () => {
  it("should convert Response to SSE stream", async () => {
    const sseText = 'data: {"test": "value1"}\n\ndata: {"test": "value2"}\n\n';
    const response = createMockResponse(sseText);

    const events: SSEEvent[] = [];
    for await (const event of responseToSSEStream(response)) {
      events.push(event);
    }

    expect(events).toHaveLength(2);
    expect(events[0].data).toBe('{"test": "value1"}');
    expect(events[1].data).toBe('{"test": "value2"}');
  });

  it("should handle response with no body", async () => {
    const response = new Response(null);

    await expect(async () => {
      for await (const event of responseToSSEStream(response)) {
        // Should not reach here
      }
    }).rejects.toThrow("Response body is null");
  });

  it("should handle chunked response", async () => {
    const chunks = ['data: {"test":', ' "value"}\n\n'];
    const response = createMockChunkedResponse(chunks);

    const events: SSEEvent[] = [];
    for await (const event of responseToSSEStream(response)) {
      events.push(event);
    }

    expect(events).toHaveLength(1);
    expect(events[0].data).toBe('{"test": "value"}');
  });
});

describe("streamChatCompletionFromRouter", () => {
  it("should throw error for failed request", async () => {
    const routerUrl = "http://localhost:3000";
    const request = {
      model: "test-model",
      messages: [{ role: "user", content: "Hello" }],
    };

    // Mock fetch to return error
    global.fetch = jest.fn().mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: "Internal Server Error",
    });

    await expect(async () => {
      for await (const chunk of streamChatCompletionFromRouter(
        routerUrl,
        request,
      )) {
        // Should not reach here
      }
    }).rejects.toThrow(
      "Circuit Breaker router request failed: 500 Internal Server Error",
    );
  });

  it("should stream completion chunks", async () => {
    const routerUrl = "http://localhost:3000";
    const request = {
      model: "test-model",
      messages: [{ role: "user", content: "Hello" }],
    };

    const sseText = `data: ${JSON.stringify({
      id: "chatcmpl-123",
      object: "chat.completion.chunk",
      created: 1677652288,
      model: "test-model",
      choices: [
        {
          index: 0,
          delta: { content: "Hello" },
          finish_reason: null,
        },
      ],
    })}\n\ndata: [DONE]\n\n`;

    // Mock fetch to return SSE stream
    global.fetch = jest.fn().mockResolvedValueOnce({
      ok: true,
      body: createMockReadableStream(sseText),
    });

    const chunks: StreamingChunk[] = [];
    for await (const chunk of streamChatCompletionFromRouter(
      routerUrl,
      request,
    )) {
      chunks.push(chunk);
    }

    expect(chunks).toHaveLength(1);
    expect(chunks[0].choices[0].delta.content).toBe("Hello");
  });

  it("should include custom headers", async () => {
    const routerUrl = "http://localhost:3000";
    const request = {
      model: "test-model",
      messages: [{ role: "user", content: "Hello" }],
    };

    const mockFetch = jest.fn().mockResolvedValueOnce({
      ok: true,
      body: createMockReadableStream("data: [DONE]\n\n"),
    });
    global.fetch = mockFetch;

    const chunks: StreamingChunk[] = [];
    for await (const chunk of streamChatCompletionFromRouter(
      routerUrl,
      request,
      {
        headers: { Authorization: "Bearer test-token" },
      },
    )) {
      chunks.push(chunk);
    }

    expect(mockFetch).toHaveBeenCalledWith(
      `${routerUrl}/v1/chat/completions`,
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer test-token",
        }),
      }),
    );
  });
});

describe("Error classes", () => {
  it("should create SSEError", () => {
    const error = new SSEError("Test error");
    expect(error.name).toBe("SSEError");
    expect(error.message).toBe("Test error");
    expect(error instanceof Error).toBe(true);
  });

  it("should create SSEParseError", () => {
    const error = new SSEParseError("Parse error", "raw data");
    expect(error.name).toBe("SSEParseError");
    expect(error.message).toBe("Parse error");
    expect(error.rawData).toBe("raw data");
    expect(error instanceof SSEError).toBe(true);
  });

  it("should create SSEStreamError", () => {
    const error = new SSEStreamError("Stream error", 500);
    expect(error.name).toBe("SSEStreamError");
    expect(error.message).toBe("Stream error");
    expect(error.statusCode).toBe(500);
    expect(error instanceof SSEError).toBe(true);
  });
});

describe("Utility functions", () => {
  describe("isSSESupported", () => {
    it("should return true in test environment", () => {
      // In Jest, fetch is available
      expect(isSSESupported()).toBe(true);
    });
  });

  describe("createSSEClient", () => {
    it("should return null when EventSource is not available", () => {
      const client = createSSEClient("http://localhost:3000/stream");
      // EventSource is not available in Jest environment
      expect(client).toBeNull();
    });
  });
});

// Helper functions for tests

function createMockResponse(text: string): Response {
  const stream = createMockReadableStream(text);
  return new Response(stream, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
    },
  });
}

function createMockChunkedResponse(chunks: string[]): Response {
  const stream = new ReadableStream({
    start(controller) {
      chunks.forEach((chunk) => {
        controller.enqueue(new TextEncoder().encode(chunk));
      });
      controller.close();
    },
  });

  return new Response(stream, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
    },
  });
}

function createMockReadableStream(text: string): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(text));
      controller.close();
    },
  });
}
