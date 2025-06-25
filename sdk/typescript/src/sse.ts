/**
 * Server-Sent Events (SSE) parsing utilities for Circuit Breaker TypeScript SDK
 *
 * This module provides utilities for parsing SSE streams from the Circuit Breaker router.
 * The router presents an OpenAI-compatible API, so we only need to handle OpenAI format.
 */

/**
 * SSE event structure
 */
export interface SSEEvent {
  eventType?: string;
  data: string;
  id?: string;
  retry?: number;
}

/**
 * Streaming chunk structure that matches OpenAI format
 */
export interface StreamingChunk {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: StreamingChoice[];
  system_fingerprint?: string;
}

/**
 * Individual choice in a streaming response
 */
export interface StreamingChoice {
  index: number;
  delta: {
    role?: string;
    content?: string;
    tool_calls?: any[];
  };
  finish_reason?: string | null;
  logprobs?: any;
}

/**
 * SSE stream parser that converts text chunks into SSE events
 */
export class SSEParser {
  private buffer: string = "";

  /**
   * Parse text chunk into SSE events
   */
  parseChunk(chunk: string): SSEEvent[] {
    this.buffer += chunk;
    const events: SSEEvent[] = [];

    // Split buffer by double newlines (event boundaries)
    while (this.buffer.includes("\n\n")) {
      const doubleNewlineIndex = this.buffer.indexOf("\n\n");
      const eventBlock = this.buffer.slice(0, doubleNewlineIndex);
      this.buffer = this.buffer.slice(doubleNewlineIndex + 2);

      if (eventBlock.trim()) {
        const event = this.parseEventBlock(eventBlock);
        if (event) {
          events.push(event);
        }
      }
    }

    return events;
  }

  /**
   * Parse a single event block into an SSE event
   */
  private parseEventBlock(block: string): SSEEvent | null {
    const lines = block.split("\n");
    const event: Partial<SSEEvent> = {};

    for (const line of lines) {
      const trimmed = line.trim();

      // Skip comments and empty lines
      if (!trimmed || trimmed.startsWith("#")) {
        continue;
      }

      if (trimmed.startsWith("data: ")) {
        event.data = trimmed.slice(6);
      } else if (trimmed.startsWith("event: ")) {
        event.eventType = trimmed.slice(7);
      } else if (trimmed.startsWith("id: ")) {
        event.id = trimmed.slice(4);
      } else if (trimmed.startsWith("retry: ")) {
        const retryValue = parseInt(trimmed.slice(7), 10);
        if (!isNaN(retryValue)) {
          event.retry = retryValue;
        }
      }
    }

    return event.data ? (event as SSEEvent) : null;
  }

  /**
   * Check if there's remaining data in the buffer
   */
  hasRemainingData(): boolean {
    return this.buffer.trim().length > 0;
  }

  /**
   * Flush any remaining data in the buffer
   */
  flushRemaining(): SSEEvent[] {
    if (!this.hasRemainingData()) {
      return [];
    }

    const event = this.parseEventBlock(this.buffer);
    this.buffer = "";
    return event ? [event] : [];
  }
}

/**
 * Convert a Response with SSE stream to an async generator
 */
export async function* responseToSSEStream(
  response: Response,
): AsyncGenerator<SSEEvent, void, unknown> {
  if (!response.body) {
    throw new Error("Response body is null");
  }

  const parser = new SSEParser();
  const reader = response.body.getReader();
  const decoder = new TextDecoder();

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const chunk = decoder.decode(value, { stream: true });
      const events = parser.parseChunk(chunk);

      for (const event of events) {
        yield event;
      }
    }

    // Process any remaining data
    const remainingEvents = parser.flushRemaining();
    for (const event of remainingEvents) {
      yield event;
    }
  } finally {
    reader.releaseLock();
  }
}

/**
 * Parse Circuit Breaker SSE event into streaming chunk
 * The router always returns OpenAI-compatible format
 */
export function parseCircuitBreakerEvent(
  event: SSEEvent,
): StreamingChunk | null {
  // Skip non-data events and completion markers
  if (
    !event.data ||
    event.data.trim() === "" ||
    event.data.trim() === "[DONE]"
  ) {
    return null;
  }

  try {
    // Circuit Breaker router returns standard OpenAI format
    const streamChunk = JSON.parse(event.data) as StreamingChunk;

    // Validate the structure matches what we expect
    if (
      !streamChunk.id ||
      !streamChunk.choices ||
      !Array.isArray(streamChunk.choices)
    ) {
      console.warn("Invalid streaming chunk format:", event.data);
      return null;
    }

    return {
      id: streamChunk.id,
      object: streamChunk.object || "chat.completion.chunk",
      created: streamChunk.created || Math.floor(Date.now() / 1000),
      model: streamChunk.model,
      choices: streamChunk.choices.map((choice) => ({
        index: choice.index || 0,
        delta: {
          role: choice.delta.role,
          content: choice.delta.content,
          tool_calls: choice.delta.tool_calls,
        },
        finish_reason: choice.finish_reason,
        logprobs: choice.logprobs,
      })),
      system_fingerprint: streamChunk.system_fingerprint,
    };
  } catch (error) {
    if (error instanceof SyntaxError) {
      console.warn(
        "Failed to parse Circuit Breaker SSE JSON:",
        error,
        "Raw data:",
        event.data,
      );
      return null;
    }
    throw error;
  }
}

/**
 * Create an async generator that streams chat completion chunks from Circuit Breaker
 */
export async function* streamChatCompletionFromRouter(
  routerUrl: string,
  request: any,
  options?: {
    headers?: Record<string, string>;
    timeout?: number;
  },
): AsyncGenerator<StreamingChunk, void, unknown> {
  const response = await fetch(`${routerUrl}/v1/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "text/event-stream",
      "Cache-Control": "no-cache",
      ...options?.headers,
    },
    body: JSON.stringify({
      ...request,
      stream: true,
    }),
    signal: options?.timeout ? AbortSignal.timeout(options.timeout) : undefined,
  });

  if (!response.ok) {
    let errorBody = "Unknown error";
    try {
      errorBody = await response.text();
      console.log("🔧 Error response body:", errorBody);
    } catch (e) {
      console.log("🔧 Could not read error response body");
    }

    throw new SSEStreamError(
      `Circuit Breaker router request failed: ${response.status} ${response.statusText}`,
      response.status,
    );
  }

  // Process SSE stream
  for await (const event of responseToSSEStream(response)) {
    const chunk = parseCircuitBreakerEvent(event);
    if (chunk) {
      yield chunk;
    }
  }
}

/**
 * Error types for SSE parsing
 */
export class SSEError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SSEError";
  }
}

export class SSEParseError extends SSEError {
  constructor(
    message: string,
    public readonly rawData?: string,
  ) {
    super(message);
    this.name = "SSEParseError";
  }
}

export class SSEStreamError extends SSEError {
  constructor(
    message: string,
    public readonly statusCode?: number,
  ) {
    super(message);
    this.name = "SSEStreamError";
  }
}

/**
 * Utility function to check if SSE is supported in the current environment
 */
export function isSSESupported(): boolean {
  return typeof EventSource !== "undefined" || typeof fetch !== "undefined";
}

/**
 * Create a simple EventSource-based SSE client for environments that support it
 */
export function createSSEClient(
  url: string,
  options?: {
    headers?: Record<string, string>;
    withCredentials?: boolean;
  },
): EventSource | null {
  if (typeof EventSource === "undefined") {
    return null;
  }

  // Note: EventSource doesn't support custom headers in most browsers
  // For authenticated requests, use the fetch-based streaming instead
  return new EventSource(url, {
    withCredentials: options?.withCredentials,
  });
}
