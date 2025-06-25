/**
 * LLM API client for Circuit Breaker TypeScript SDK
 */
import { ChatMessage, ChatCompletionRequest, ChatCompletionResponse, PaginatedResult } from "./types.js";
import type { Client } from "./client.js";
export declare class LLMClient {
    private client;
    constructor(client: Client);
    /**
     * Send a chat completion request
     */
    chatCompletion(request: ChatCompletionRequest): Promise<ChatCompletionResponse>;
    /**
     * Simple chat method for single message
     */
    chat(model: string, message: string, options?: {
        systemPrompt?: string;
        temperature?: number;
        maxTokens?: number;
    }): Promise<string>;
    /**
     * List available models
     */
    listModels(): Promise<PaginatedResult<LLMModel>>;
    /**
     * Get model information
     */
    getModel(modelId: string): Promise<LLMModel>;
    /**
     * Stream chat completion (if supported by server)
     */
    streamChatCompletion(request: ChatCompletionRequest & {
        stream: true;
    }, onChunk: (chunk: ChatCompletionChunk) => void): Promise<void>;
    /**
     * Count tokens in text (approximate)
     */
    countTokens(model: string, text: string): Promise<TokenCount>;
    /**
     * Get provider health status
     */
    getProviderHealth(): Promise<ProviderHealth[]>;
}
/**
 * LLM Model information
 */
export interface LLMModel {
    id: string;
    name: string;
    provider: string;
    max_tokens: number;
    supports_streaming: boolean;
    supports_functions: boolean;
    cost_per_1k_tokens?: {
        input: number;
        output: number;
    };
}
/**
 * Chat completion chunk for streaming
 */
export interface ChatCompletionChunk {
    id: string;
    choices: ChoiceDelta[];
    model: string;
    created: number;
}
export interface ChoiceDelta {
    index: number;
    delta: MessageDelta;
    finish_reason?: string;
}
export interface MessageDelta {
    role?: "assistant";
    content?: string;
}
/**
 * Token count response
 */
export interface TokenCount {
    tokens: number;
    estimated: boolean;
}
/**
 * Provider health information
 */
export interface ProviderHealth {
    provider: string;
    status: "healthy" | "degraded" | "unhealthy";
    response_time_ms?: number;
    error_rate?: number;
    last_check: string;
}
/**
 * Simple chat builder for multi-turn conversations
 */
export declare class ChatBuilder {
    private model;
    private messages;
    private temperature?;
    private maxTokens?;
    constructor(model: string);
    /**
     * Set system prompt
     */
    setSystemPrompt(prompt: string): ChatBuilder;
    /**
     * Add user message
     */
    addUserMessage(content: string): ChatBuilder;
    /**
     * Add assistant message
     */
    addAssistantMessage(content: string): ChatBuilder;
    /**
     * Set temperature
     */
    setTemperature(temperature: number): ChatBuilder;
    /**
     * Set max tokens
     */
    setMaxTokens(maxTokens: number): ChatBuilder;
    /**
     * Build the chat completion request
     */
    build(): ChatCompletionRequest;
    /**
     * Execute the chat completion
     */
    execute(client: LLMClient): Promise<ChatCompletionResponse>;
    /**
     * Execute and return just the content
     */
    getResponse(client: LLMClient): Promise<string>;
}
/**
 * Create a new chat builder
 */
export declare function createChat(model: string): ChatBuilder;
/**
 * Quick chat function for simple use cases
 */
export declare function quickChat(client: LLMClient, model: string, prompt: string, options?: {
    systemPrompt?: string;
    temperature?: number;
    maxTokens?: number;
}): Promise<string>;
/**
 * Common model constants
 */
export declare const COMMON_MODELS: {
    readonly GPT_3_5_TURBO: "gpt-3.5-turbo";
    readonly GPT_4: "o4-mini-2025-04-16";
    readonly GPT_4O_MINI: "o4-mini-2025-04-16";
    readonly GPT_4_TURBO: "gpt-4-turbo-preview";
    readonly CLAUDE_3_HAIKU: "claude-3-haiku-20240307";
    readonly CLAUDE_3_SONNET: "claude-3-sonnet-20240229";
    readonly CLAUDE_3_OPUS: "claude-3-opus-20240229";
    readonly LLAMA_2_7B: "llama2:7b";
    readonly LLAMA_2_13B: "llama2:13b";
    readonly LLAMA_2_70B: "llama2:70b";
};
/**
 * Create a conversation with context management
 */
export declare class Conversation {
    private client;
    private model;
    private messages;
    private systemPrompt?;
    private maxContextLength;
    private temperature?;
    private maxTokens?;
    constructor(client: LLMClient, model: string, options?: {
        systemPrompt?: string;
        maxContextLength?: number;
        temperature?: number;
        maxTokens?: number;
    });
    /**
     * Send a message and get response
     */
    send(message: string): Promise<string>;
    /**
     * Get conversation history
     */
    getHistory(): ChatMessage[];
    /**
     * Clear conversation history (keep system prompt)
     */
    clear(): void;
    /**
     * Simple context truncation (remove oldest messages)
     */
    private truncateContext;
}
/**
 * Create a new conversation
 */
export declare function createConversation(client: LLMClient, model: string, options?: {
    systemPrompt?: string;
    maxContextLength?: number;
    temperature?: number;
    maxTokens?: number;
}): Conversation;
//# sourceMappingURL=llm.d.ts.map