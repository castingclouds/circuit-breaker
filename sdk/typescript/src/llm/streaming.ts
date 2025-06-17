/**
 * Streaming Handler for Circuit Breaker SDK
 *
 * Provides real-time streaming capabilities for LLM responses with features like:
 * - Server-Sent Events (SSE) support
 * - WebSocket streaming
 * - Chunk aggregation and buffering
 * - Error handling and reconnection
 * - Stream interruption and cancellation
 *
 * @example
 * ```typescript
 * const handler = new StreamingHandler();
 *
 * const stream = handler.createStream();
 * stream.on('chunk', (chunk) => {
 *   console.log('Received:', chunk.choices[0]?.delta?.content);
 * });
 *
 * stream.on('complete', (response) => {
 *   console.log('Stream completed:', response);
 * });
 *
 * // Start streaming
 * await handler.streamChatCompletion(request, stream);
 * ```
 */

import { EventEmitter } from 'events';
import {
  ChatCompletionRequest,
  ChatCompletionResponse,
  ChatCompletionChunk,
  ChatMessage,
  Usage,
} from '../core/types.js';
import {
  LLMError,
  LLMProviderError,
  NetworkError,
  TimeoutError,
} from '../core/errors.js';
import { Logger, createComponentLogger } from '../utils/logger.js';

export interface StreamConfig {
  /** Buffer size for chunk aggregation */
  bufferSize?: number;

  /** Timeout for individual chunks (ms) */
  chunkTimeout?: number;

  /** Overall stream timeout (ms) */
  streamTimeout?: number;

  /** Auto-reconnect on connection loss */
  autoReconnect?: boolean;

  /** Maximum reconnection attempts */
  maxReconnectAttempts?: number;

  /** Reconnection delay (ms) */
  reconnectDelay?: number;

  /** Enable chunk validation */
  validateChunks?: boolean;

  /** Custom chunk processor */
  chunkProcessor?: (chunk: ChatCompletionChunk) => ChatCompletionChunk;
}

export interface StreamStats {
  startTime: Date;
  endTime?: Date;
  totalChunks: number;
  totalTokens: number;
  averageChunkSize: number;
  latency: number;
  reconnectCount: number;
  errorCount: number;
}

export interface StreamEvent {
  type: 'chunk' | 'complete' | 'error' | 'timeout' | 'abort';
  data?: any;
  timestamp: Date;
}

/**
 * Streaming session for managing individual stream instances
 */
export class StreamingSession extends EventEmitter {
  public readonly id: string;
  public readonly config: StreamConfig;
  private logger: Logger;
  private stats: StreamStats;
  private buffer: ChatCompletionChunk[] = [];
  private aggregatedResponse: Partial<ChatCompletionResponse> = {};
  private abortController?: AbortController;
  private timeoutHandle?: NodeJS.Timeout;
  private reconnectAttempts = 0;
  private isActive = false;

  constructor(id: string, config: StreamConfig = {}, logger?: Logger) {
    super();

    this.id = id;
    this.config = {
      bufferSize: 10,
      chunkTimeout: 5000,
      streamTimeout: 30000,
      autoReconnect: true,
      maxReconnectAttempts: 3,
      reconnectDelay: 1000,
      validateChunks: true,
      ...config,
    };

    this.logger = logger || createComponentLogger(`StreamSession:${this.id}`);

    this.stats = {
      startTime: new Date(),
      totalChunks: 0,
      totalTokens: 0,
      averageChunkSize: 0,
      latency: 0,
      reconnectCount: 0,
      errorCount: 0,
    };

    this.setupTimeouts();
  }

  /**
   * Start the streaming session
   */
  start(): void {
    if (this.isActive) {
      throw new Error('Stream session is already active');
    }

    this.isActive = true;
    this.abortController = new AbortController();
    this.stats.startTime = new Date();

    this.logger.debug('Stream session started', { id: this.id });
    this.emit('start', { sessionId: this.id });
  }

  /**
   * Process incoming chunk
   */
  processChunk(chunk: ChatCompletionChunk): void {
    if (!this.isActive) {
      this.logger.warn('Received chunk for inactive session', { id: this.id });
      return;
    }

    try {
      // Validate chunk if enabled
      if (this.config.validateChunks && !this.validateChunk(chunk)) {
        throw new Error('Invalid chunk format');
      }

      // Apply custom chunk processor if provided
      const processedChunk = this.config.chunkProcessor
        ? this.config.chunkProcessor(chunk)
        : chunk;

      // Update statistics
      this.updateStats(processedChunk);

      // Add to buffer
      this.buffer.push(processedChunk);

      // Emit chunk event
      this.emit('chunk', processedChunk);

      // Aggregate response
      this.aggregateChunk(processedChunk);

      // Check if stream is complete
      if (this.isStreamComplete(processedChunk)) {
        this.complete();
      }

      // Flush buffer if needed
      if (this.buffer.length >= this.config.bufferSize!) {
        this.flushBuffer();
      }

      // Reset chunk timeout
      this.resetChunkTimeout();

    } catch (error) {
      this.handleError(error as Error);
    }
  }

  /**
   * Complete the streaming session
   */
  complete(): void {
    if (!this.isActive) return;

    this.isActive = false;
    this.stats.endTime = new Date();
    this.stats.latency = this.stats.endTime.getTime() - this.stats.startTime.getTime();

    // Flush remaining buffer
    this.flushBuffer();

    // Build final response
    const finalResponse = this.buildFinalResponse();

    this.logger.debug('Stream session completed', {
      id: this.id,
      stats: this.stats,
      totalChunks: this.stats.totalChunks,
    });

    this.emit('complete', finalResponse);
    this.cleanup();
  }

  /**
   * Abort the streaming session
   */
  abort(): void {
    if (!this.isActive) return;

    this.isActive = false;
    this.abortController?.abort();

    this.logger.debug('Stream session aborted', { id: this.id });
    this.emit('abort', { sessionId: this.id });
    this.cleanup();
  }

  /**
   * Handle stream error
   */
  handleError(error: Error): void {
    this.stats.errorCount++;

    this.logger.error('Stream session error', {
      id: this.id,
      error: error.message,
      stack: error.stack,
    });

    // Attempt reconnection if configured
    if (this.config.autoReconnect && this.shouldReconnect()) {
      this.attemptReconnect();
    } else {
      this.emit('error', error);
      this.abort();
    }
  }

  /**
   * Get current stream statistics
   */
  getStats(): StreamStats {
    return { ...this.stats };
  }

  /**
   * Check if session is active
   */
  isActiveSession(): boolean {
    return this.isActive;
  }

  /**
   * Get abort signal for fetch requests
   */
  getAbortSignal(): AbortSignal | undefined {
    return this.abortController?.signal;
  }

  /**
   * Validate chunk format
   */
  private validateChunk(chunk: ChatCompletionChunk): boolean {
    return (
      chunk &&
      typeof chunk === 'object' &&
      typeof chunk.id === 'string' &&
      typeof chunk.object === 'string' &&
      Array.isArray(chunk.choices)
    );
  }

  /**
   * Update session statistics
   */
  private updateStats(chunk: ChatCompletionChunk): void {
    this.stats.totalChunks++;

    // Update token count if available
    if (chunk.usage) {
      this.stats.totalTokens += chunk.usage.total_tokens || 0;
    }

    // Update average chunk size
    const chunkSize = JSON.stringify(chunk).length;
    this.stats.averageChunkSize =
      (this.stats.averageChunkSize * (this.stats.totalChunks - 1) + chunkSize) /
      this.stats.totalChunks;
  }

  /**
   * Aggregate chunk into final response
   */
  private aggregateChunk(chunk: ChatCompletionChunk): void {
    if (!this.aggregatedResponse.id) {
      this.aggregatedResponse = {
        id: chunk.id,
        object: 'chat.completion',
        created: chunk.created,
        model: chunk.model,
        choices: [],
        usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
      };
    }

    // Aggregate choices
    for (const choice of chunk.choices) {
      const existingChoice = this.aggregatedResponse.choices![choice.index];

      if (!existingChoice) {
        this.aggregatedResponse.choices![choice.index] = {
          index: choice.index,
          message: {
            role: choice.delta.role || 'assistant',
            content: choice.delta.content || '',
          },
          finish_reason: choice.finish_reason,
        };
      } else {
        // Append content
        if (choice.delta.content) {
          existingChoice.message.content += choice.delta.content;
        }

        // Update finish reason
        if (choice.finish_reason) {
          existingChoice.finish_reason = choice.finish_reason;
        }
      }
    }

    // Update usage if available
    if (chunk.usage) {
      const usage = this.aggregatedResponse.usage!;
      usage.prompt_tokens = chunk.usage.prompt_tokens || usage.prompt_tokens;
      usage.completion_tokens = chunk.usage.completion_tokens || usage.completion_tokens;
      usage.total_tokens = chunk.usage.total_tokens || usage.total_tokens;
    }
  }

  /**
   * Check if stream is complete
   */
  private isStreamComplete(chunk: ChatCompletionChunk): boolean {
    return chunk.choices.some(choice => choice.finish_reason !== null);
  }

  /**
   * Flush buffer
   */
  private flushBuffer(): void {
    if (this.buffer.length > 0) {
      this.emit('buffer', [...this.buffer]);
      this.buffer = [];
    }
  }

  /**
   * Build final response
   */
  private buildFinalResponse(): ChatCompletionResponse {
    return {
      id: this.aggregatedResponse.id!,
      object: 'chat.completion',
      created: this.aggregatedResponse.created!,
      model: this.aggregatedResponse.model!,
      choices: this.aggregatedResponse.choices!,
      usage: this.aggregatedResponse.usage!,
    };
  }

  /**
   * Setup timeouts
   */
  private setupTimeouts(): void {
    // Overall stream timeout
    if (this.config.streamTimeout) {
      setTimeout(() => {
        if (this.isActive) {
          this.handleError(new TimeoutError('Stream timeout exceeded'));
        }
      }, this.config.streamTimeout);
    }
  }

  /**
   * Reset chunk timeout
   */
  private resetChunkTimeout(): void {
    if (this.timeoutHandle) {
      clearTimeout(this.timeoutHandle);
    }

    if (this.config.chunkTimeout) {
      this.timeoutHandle = setTimeout(() => {
        if (this.isActive) {
          this.handleError(new TimeoutError('Chunk timeout exceeded'));
        }
      }, this.config.chunkTimeout);
    }
  }

  /**
   * Check if should attempt reconnection
   */
  private shouldReconnect(): boolean {
    return (
      this.config.autoReconnect! &&
      this.reconnectAttempts < this.config.maxReconnectAttempts!
    );
  }

  /**
   * Attempt reconnection
   */
  private attemptReconnect(): void {
    this.reconnectAttempts++;
    this.stats.reconnectCount++;

    this.logger.info('Attempting reconnection', {
      id: this.id,
      attempt: this.reconnectAttempts,
      maxAttempts: this.config.maxReconnectAttempts,
    });

    setTimeout(() => {
      this.emit('reconnect', {
        sessionId: this.id,
        attempt: this.reconnectAttempts,
      });
    }, this.config.reconnectDelay! * this.reconnectAttempts);
  }

  /**
   * Cleanup resources
   */
  private cleanup(): void {
    if (this.timeoutHandle) {
      clearTimeout(this.timeoutHandle);
    }

    this.abortController = undefined;
    this.removeAllListeners();
  }
}

/**
 * Main streaming handler class
 */
export class StreamingHandler extends EventEmitter {
  private logger: Logger;
  private activeSessions: Map<string, StreamingSession> = new Map();
  private sessionCounter = 0;

  constructor(logger?: Logger) {
    super();
    this.logger = logger || createComponentLogger('StreamingHandler');
  }

  /**
   * Create a new streaming session
   */
  createStream(config?: StreamConfig): StreamingSession {
    const sessionId = `stream_${++this.sessionCounter}_${Date.now()}`;
    const session = new StreamingSession(sessionId, config, this.logger);

    // Set up session event handlers
    this.setupSessionHandlers(session);

    this.activeSessions.set(sessionId, session);
    this.logger.debug('Created streaming session', { sessionId });

    return session;
  }

  /**
   * Get active session by ID
   */
  getSession(sessionId: string): StreamingSession | undefined {
    return this.activeSessions.get(sessionId);
  }

  /**
   * Get all active sessions
   */
  getActiveSessions(): StreamingSession[] {
    return Array.from(this.activeSessions.values());
  }

  /**
   * Abort all active sessions
   */
  abortAllSessions(): void {
    for (const session of this.activeSessions.values()) {
      session.abort();
    }
    this.activeSessions.clear();
  }

  /**
   * Get streaming statistics
   */
  getStreamingStats(): {
    totalSessions: number;
    activeSessions: number;
    completedSessions: number;
    errorSessions: number;
  } {
    const activeSessions = this.activeSessions.size;
    const totalSessions = this.sessionCounter;

    return {
      totalSessions,
      activeSessions,
      completedSessions: totalSessions - activeSessions,
      errorSessions: 0, // Could be tracked separately
    };
  }

  /**
   * Process Server-Sent Events stream
   */
  async processSSEStream(
    response: Response,
    session: StreamingSession
  ): Promise<void> {
    if (!response.body) {
      throw new LLMProviderError('SSE', 'No response body for streaming');
    }

    const decoder = new TextDecoder();
    const reader = response.body.getReader();

    try {
      session.start();

      while (session.isActiveSession()) {
        const { done, value } = await reader.read();

        if (done) {
          session.complete();
          break;
        }

        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split('\n').filter(line => line.trim() !== '');

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6);

            if (data === '[DONE]') {
              session.complete();
              return;
            }

            try {
              const parsed = JSON.parse(data) as ChatCompletionChunk;
              session.processChunk(parsed);
            } catch (error) {
              this.logger.warn('Failed to parse SSE chunk', { line, error });
            }
          }
        }
      }
    } catch (error) {
      session.handleError(error as Error);
    } finally {
      reader.releaseLock();
    }
  }

  /**
   * Process WebSocket stream
   */
  async processWebSocketStream(
    ws: WebSocket,
    session: StreamingSession
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      session.start();

      ws.onmessage = (event) => {
        try {
          const chunk = JSON.parse(event.data) as ChatCompletionChunk;
          session.processChunk(chunk);
        } catch (error) {
          session.handleError(error as Error);
        }
      };

      ws.onclose = () => {
        session.complete();
        resolve();
      };

      ws.onerror = (error) => {
        session.handleError(new NetworkError('WebSocket error'));
        reject(error);
      };
    });
  }

  /**
   * Create streaming response aggregator
   */
  createAggregator(): {
    add: (chunk: ChatCompletionChunk) => void;
    getResult: () => ChatCompletionResponse | null;
    clear: () => void;
  } {
    let aggregated: Partial<ChatCompletionResponse> = {};

    return {
      add: (chunk: ChatCompletionChunk) => {
        if (!aggregated.id) {
          aggregated = {
            id: chunk.id,
            object: 'chat.completion',
            created: chunk.created,
            model: chunk.model,
            choices: [],
            usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
          };
        }

        // Aggregate choices
        for (const choice of chunk.choices) {
          const existingChoice = aggregated.choices![choice.index];

          if (!existingChoice) {
            aggregated.choices![choice.index] = {
              index: choice.index,
              message: {
                role: choice.delta.role || 'assistant',
                content: choice.delta.content || '',
              },
              finish_reason: choice.finish_reason,
            };
          } else {
            if (choice.delta.content) {
              existingChoice.message.content += choice.delta.content;
            }
            if (choice.finish_reason) {
              existingChoice.finish_reason = choice.finish_reason;
            }
          }
        }
      },

      getResult: () => {
        if (!aggregated.id) return null;
        return aggregated as ChatCompletionResponse;
      },

      clear: () => {
        aggregated = {};
      },
    };
  }

  /**
   * Setup session event handlers
   */
  private setupSessionHandlers(session: StreamingSession): void {
    session.on('start', (data) => {
      this.emit('sessionStart', data);
    });

    session.on('chunk', (chunk) => {
      this.emit('chunk', { sessionId: session.id, chunk });
    });

    session.on('complete', (response) => {
      this.activeSessions.delete(session.id);
      this.emit('sessionComplete', { sessionId: session.id, response });
    });

    session.on('abort', (data) => {
      this.activeSessions.delete(session.id);
      this.emit('sessionAbort', data);
    });

    session.on('error', (error) => {
      this.activeSessions.delete(session.id);
      this.emit('sessionError', { sessionId: session.id, error });
    });

    session.on('reconnect', (data) => {
      this.emit('sessionReconnect', data);
    });
  }
}

/**
 * Create a streaming handler instance
 */
export function createStreamingHandler(logger?: Logger): StreamingHandler {
  return new StreamingHandler(logger);
}

/**
 * Utility function to convert async generator to streaming session
 */
export async function streamFromAsyncGenerator(
  generator: AsyncGenerator<ChatCompletionChunk>,
  handler: StreamingHandler,
  config?: StreamConfig
): Promise<ChatCompletionResponse> {
  const session = handler.createStream(config);

  return new Promise((resolve, reject) => {
    session.on('complete', resolve);
    session.on('error', reject);

    (async () => {
      try {
        session.start();

        for await (const chunk of generator) {
          session.processChunk(chunk);
        }

        if (session.isActiveSession()) {
          session.complete();
        }
      } catch (error) {
        session.handleError(error as Error);
      }
    })();
  });
}

/**
 * Stream processing utilities
 */
export const StreamUtils = {
  /**
   * Extract text content from streaming chunks
   */
  extractContent: (chunks: ChatCompletionChunk[]): string => {
    return chunks
      .flatMap(chunk => chunk.choices)
      .filter(choice => choice.delta.content)
      .map(choice => choice.delta.content)
      .join('');
  },

  /**
   * Check if stream contains errors
   */
  hasErrors: (chunks: ChatCompletionChunk[]): boolean => {
    return chunks.some(chunk =>
      chunk.choices.some(choice => choice.finish_reason === 'error')
    );
  },

  /**
   * Get completion reason from chunks
   */
  getFinishReason: (chunks: ChatCompletionChunk[]): string | null => {
    const reasons = chunks
      .flatMap(chunk => chunk.choices)
      .map(choice => choice.finish_reason)
      .filter(reason => reason !== null);

    return reasons.length > 0 ? reasons[reasons.length - 1] : null;
  },

  /**
   * Calculate streaming statistics
   */
  calculateStats: (chunks: ChatCompletionChunk[]): {
    totalChunks: number;
    totalContent: number;
    averageChunkSize: number;
  } => {
    const totalContent = StreamUtils.extractContent(chunks).length;
    const totalChunks = chunks.length;
    const averageChunkSize = totalChunks > 0 ? totalContent / totalChunks : 0;

    return {
      totalChunks,
      totalContent,
      averageChunkSize,
    };
  },
};
