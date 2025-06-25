/**
 * Core types for the Circuit Breaker TypeScript SDK
 */
// ============================================================================
// Error Types
// ============================================================================
export class CircuitBreakerError extends Error {
    constructor(message, code, details) {
        super(message);
        this.code = code;
        this.details = details;
        this.name = "CircuitBreakerError";
    }
}
export class NetworkError extends CircuitBreakerError {
    constructor(message, details) {
        super(message, "NETWORK_ERROR", details);
        this.name = "NetworkError";
    }
}
export class ValidationError extends CircuitBreakerError {
    constructor(message, details) {
        super(message, "VALIDATION_ERROR", details);
        this.name = "ValidationError";
    }
}
export class NotFoundError extends CircuitBreakerError {
    constructor(resource, details) {
        super(`${resource} not found`, "NOT_FOUND", details);
        this.name = "NotFoundError";
    }
}
//# sourceMappingURL=types.js.map