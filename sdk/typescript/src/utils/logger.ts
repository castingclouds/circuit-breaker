/**
 * Logging utility for Circuit Breaker TypeScript SDK
 *
 * This file provides a flexible logging system with multiple output formats,
 * log levels, and structured logging support.
 */

import { LoggingConfig } from '../core/types.js';

// ============================================================================
// Types
// ============================================================================

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface LogEntry {
  timestamp: Date;
  level: LogLevel;
  message: string;
  meta?: any;
  component?: string;
  requestId?: string;
  userId?: string;
  correlationId?: string;
}

export interface LoggerConfig extends LoggingConfig {
  component?: string;
  enableConsole?: boolean;
  enableFile?: boolean;
  filePath?: string;
  maxFileSize?: number;
  maxFiles?: number;
  enableRemote?: boolean;
  remoteEndpoint?: string;
}

// ============================================================================
// Logger Class
// ============================================================================

export class Logger {
  private readonly config: LoggerConfig;
  private readonly logBuffer: LogEntry[] = [];
  private readonly maxBufferSize = 1000;
  private logCount = 0;

  constructor(config?: LoggerConfig) {
    this.config = {
      level: 'info',
      structured: false,
      enableConsole: true,
      enableFile: false,
      enableRemote: false,
      ...config,
    };
  }

  /**
   * Log a debug message
   */
  debug(message: string, meta?: any, context?: LogContext): void {
    this.log('debug', message, meta, context);
  }

  /**
   * Log an info message
   */
  info(message: string, meta?: any, context?: LogContext): void {
    this.log('info', message, meta, context);
  }

  /**
   * Log a warning message
   */
  warn(message: string, meta?: any, context?: LogContext): void {
    this.log('warn', message, meta, context);
  }

  /**
   * Log an error message
   */
  error(message: string, meta?: any, context?: LogContext): void {
    this.log('error', message, meta, context);
  }

  /**
   * Main logging method
   */
  log(level: LogLevel, message: string, meta?: any, context?: LogContext): void {
    // Check if we should log this level
    if (!this.shouldLog(level)) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date(),
      level,
      message,
      meta,
      component: context?.component || this.config.component,
      requestId: context?.requestId,
      userId: context?.userId,
      correlationId: context?.correlationId,
    };

    // Add to buffer
    this.addToBuffer(entry);

    // Output to configured destinations
    if (this.config.enableConsole) {
      this.logToConsole(entry);
    }

    if (this.config.enableFile) {
      this.logToFile(entry);
    }

    if (this.config.enableRemote) {
      this.logToRemote(entry);
    }

    // Use custom logger if provided
    if (this.config.logger) {
      this.config.logger(level, message, meta);
    }

    this.logCount++;
  }

  /**
   * Create a child logger with additional context
   */
  child(context: Partial<LogContext>): Logger {
    const childConfig = {
      ...this.config,
      component: context.component || this.config.component,
    };

    const childLogger = new Logger(childConfig);

    // Override log method to include context
    const originalLog = childLogger.log.bind(childLogger);
    childLogger.log = (level: LogLevel, message: string, meta?: any, childContext?: LogContext) => {
      const mergedContext = { ...context, ...childContext };
      originalLog(level, message, meta, mergedContext);
    };

    return childLogger;
  }

  /**
   * Get recent log entries
   */
  getRecentLogs(count: number = 100): LogEntry[] {
    return this.logBuffer.slice(-count);
  }

  /**
   * Get log statistics
   */
  getStats(): LogStats {
    const now = Date.now();
    const oneHourAgo = now - (60 * 60 * 1000);
    const recentLogs = this.logBuffer.filter(entry => entry.timestamp.getTime() > oneHourAgo);

    const levelCounts = recentLogs.reduce((counts, entry) => {
      counts[entry.level] = (counts[entry.level] || 0) + 1;
      return counts;
    }, {} as Record<LogLevel, number>);

    return {
      totalLogs: this.logCount,
      recentLogs: recentLogs.length,
      levelCounts,
      bufferSize: this.logBuffer.length,
      oldestLogTime: this.logBuffer[0]?.timestamp,
      newestLogTime: this.logBuffer[this.logBuffer.length - 1]?.timestamp,
    };
  }

  /**
   * Clear log buffer
   */
  clearBuffer(): void {
    this.logBuffer.length = 0;
  }

  /**
   * Set log level
   */
  setLevel(level: LogLevel): void {
    (this.config as any).level = level;
  }

  /**
   * Enable/disable structured logging
   */
  setStructured(structured: boolean): void {
    (this.config as any).structured = structured;
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private shouldLog(level: LogLevel): boolean {
    const levels: Record<LogLevel, number> = {
      debug: 0,
      info: 1,
      warn: 2,
      error: 3,
    };

    return levels[level] >= levels[this.config.level!];
  }

  private addToBuffer(entry: LogEntry): void {
    this.logBuffer.push(entry);

    // Trim buffer if it gets too large
    if (this.logBuffer.length > this.maxBufferSize) {
      this.logBuffer.splice(0, this.logBuffer.length - this.maxBufferSize);
    }
  }

  private logToConsole(entry: LogEntry): void {
    if (this.config.structured) {
      const structuredEntry = {
        timestamp: entry.timestamp.toISOString(),
        level: entry.level,
        message: entry.message,
        component: entry.component,
        requestId: entry.requestId,
        userId: entry.userId,
        correlationId: entry.correlationId,
        meta: entry.meta,
      };

      console.log(JSON.stringify(structuredEntry));
    } else {
      const timestamp = entry.timestamp.toISOString();
      const level = entry.level.toUpperCase().padEnd(5);
      const component = entry.component ? `[${entry.component}]` : '';
      const requestId = entry.requestId ? `[${entry.requestId}]` : '';

      let logMessage = `${timestamp} ${level} ${component}${requestId} ${entry.message}`;

      if (entry.meta) {
        logMessage += ` ${JSON.stringify(entry.meta)}`;
      }

      // Use appropriate console method
      switch (entry.level) {
        case 'debug':
          console.debug(logMessage);
          break;
        case 'info':
          console.info(logMessage);
          break;
        case 'warn':
          console.warn(logMessage);
          break;
        case 'error':
          console.error(logMessage);
          break;
      }
    }
  }

  private logToFile(entry: LogEntry): void {
    // File logging would be implemented here
    // This would typically use fs.appendFile or a logging library
    // For now, we'll just store the intention
    if (this.config.filePath) {
      // TODO: Implement file logging
      // fs.appendFile(this.config.filePath, this.formatLogEntry(entry) + '\n');
    }
  }

  private logToRemote(entry: LogEntry): void {
    // Remote logging would be implemented here
    // This could send to services like LogDNA, Datadog, etc.
    if (this.config.remoteEndpoint) {
      // TODO: Implement remote logging
      // fetch(this.config.remoteEndpoint, { method: 'POST', body: JSON.stringify(entry) });
    }
  }

  private formatLogEntry(entry: LogEntry): string {
    if (this.config.structured) {
      return JSON.stringify({
        timestamp: entry.timestamp.toISOString(),
        level: entry.level,
        message: entry.message,
        component: entry.component,
        requestId: entry.requestId,
        userId: entry.userId,
        correlationId: entry.correlationId,
        meta: entry.meta,
      });
    } else {
      const timestamp = entry.timestamp.toISOString();
      const level = entry.level.toUpperCase().padEnd(5);
      const component = entry.component ? `[${entry.component}]` : '';
      const requestId = entry.requestId ? `[${entry.requestId}]` : '';

      let logMessage = `${timestamp} ${level} ${component}${requestId} ${entry.message}`;

      if (entry.meta) {
        logMessage += ` ${JSON.stringify(entry.meta)}`;
      }

      return logMessage;
    }
  }
}

// ============================================================================
// Helper Types and Interfaces
// ============================================================================

export interface LogContext {
  component?: string;
  requestId?: string;
  userId?: string;
  correlationId?: string;
}

export interface LogStats {
  totalLogs: number;
  recentLogs: number;
  levelCounts: Record<LogLevel, number>;
  bufferSize: number;
  oldestLogTime?: Date;
  newestLogTime?: Date;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create a logger with default configuration
 */
export function createLogger(config?: LoggerConfig): Logger {
  return new Logger(config);
}

/**
 * Create a logger for a specific component
 */
export function createComponentLogger(component: string, config?: LoggerConfig): Logger {
  return new Logger({
    ...config,
    component,
  });
}

/**
 * Format error for logging
 */
export function formatError(error: unknown): any {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack,
      ...(error as any).toJSON?.(),
    };
  }

  return { error: String(error) };
}

/**
 * Sanitize sensitive data for logging
 */
export function sanitizeForLogging(data: any): any {
  if (!data || typeof data !== 'object') {
    return data;
  }

  const sensitiveKeys = [
    'password',
    'token',
    'apiKey',
    'secret',
    'authorization',
    'credential',
    'key',
    'privateKey',
    'publicKey',
    'sessionId',
    'accessToken',
    'refreshToken',
  ];

  const sanitized = Array.isArray(data) ? [...data] : { ...data };

  for (const key of Object.keys(sanitized)) {
    if (sensitiveKeys.some(sensitive => key.toLowerCase().includes(sensitive))) {
      sanitized[key] = '[REDACTED]';
    } else if (typeof sanitized[key] === 'object' && sanitized[key] !== null) {
      sanitized[key] = sanitizeForLogging(sanitized[key]);
    }
  }

  return sanitized;
}

/**
 * Generate correlation ID for request tracking
 */
export function generateCorrelationId(): string {
  return `corr_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Generate request ID
 */
export function generateRequestId(): string {
  return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

// ============================================================================
// Default Logger Instance
// ============================================================================

export const defaultLogger = new Logger();

// ============================================================================
// Log Level Utilities
// ============================================================================

export const LOG_LEVELS: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

export function isLogLevel(level: string): level is LogLevel {
  return level in LOG_LEVELS;
}

export function compareLogLevels(level1: LogLevel, level2: LogLevel): number {
  return LOG_LEVELS[level1] - LOG_LEVELS[level2];
}
