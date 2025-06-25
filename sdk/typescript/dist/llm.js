/**
 * LLM API client for Circuit Breaker TypeScript SDK
 */
export class LLMClient {
    constructor(client) {
        this.client = client;
    }
    /**
     * Send a chat completion request
     */
    async chatCompletion(request) {
        // Use OpenAI-compatible endpoint on port 8081
        const response = await fetch("http://localhost:8081/v1/chat/completions", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(request),
        });
        if (!response.ok) {
            throw new Error(`LLM API error: ${response.status} ${response.statusText}`);
        }
        return response.json();
    }
    /**
     * Simple chat method for single message
     */
    async chat(model, message, options) {
        const messages = [];
        if (options?.systemPrompt) {
            messages.push({ role: "system", content: options.systemPrompt });
        }
        messages.push({ role: "user", content: message });
        const request = {
            model,
            messages,
            ...(options?.temperature !== undefined && {
                temperature: options.temperature,
            }),
            ...(options?.maxTokens !== undefined && {
                max_tokens: options.maxTokens,
            }),
        };
        const response = await this.chatCompletion(request);
        return response.choices[0]?.message?.content || "";
    }
    /**
     * List available models
     */
    async listModels() {
        const response = await fetch("http://localhost:8081/v1/models");
        if (!response.ok) {
            throw new Error(`LLM API error: ${response.status} ${response.statusText}`);
        }
        const data = await response.json();
        return {
            data: data.data || [],
            total: data.data?.length || 0,
            hasMore: false,
        };
    }
    /**
     * Get model information
     */
    async getModel(modelId) {
        const response = await fetch(`http://localhost:8081/v1/models/${modelId}`);
        if (!response.ok) {
            throw new Error(`LLM API error: ${response.status} ${response.statusText}`);
        }
        return response.json();
    }
    /**
     * Stream chat completion (if supported by server)
     */
    async streamChatCompletion(request, onChunk) {
        // Note: This would require server-sent events or WebSocket support
        // For now, we'll fall back to regular completion
        const response = await this.chatCompletion({ ...request, stream: false });
        // Simulate streaming by calling onChunk with the full response
        const chunk = {
            id: response.id,
            choices: response.choices.map((choice) => ({
                index: choice.index,
                delta: { content: choice.message.content },
                finish_reason: choice.finish_reason,
            })),
            model: response.model,
            created: response.created,
        };
        onChunk(chunk);
    }
    /**
     * Count tokens in text (approximate)
     */
    async countTokens(model, text) {
        const body = { model, text };
        return this.client.request("POST", "/api/llm/tokens/count", body);
    }
    /**
     * Get provider health status
     */
    async getProviderHealth() {
        return this.client.request("GET", "/api/llm/health");
    }
}
/**
 * Simple chat builder for multi-turn conversations
 */
export class ChatBuilder {
    constructor(model) {
        this.messages = [];
        this.model = model;
    }
    /**
     * Set system prompt
     */
    setSystemPrompt(prompt) {
        // Remove existing system message if any
        this.messages = this.messages.filter((m) => m.role !== "system");
        this.messages.unshift({ role: "system", content: prompt });
        return this;
    }
    /**
     * Add user message
     */
    addUserMessage(content) {
        this.messages.push({ role: "user", content });
        return this;
    }
    /**
     * Add assistant message
     */
    addAssistantMessage(content) {
        this.messages.push({ role: "assistant", content });
        return this;
    }
    /**
     * Set temperature
     */
    setTemperature(temperature) {
        if (temperature < 0 || temperature > 2) {
            throw new Error("Temperature must be between 0 and 2");
        }
        this.temperature = temperature;
        return this;
    }
    /**
     * Set max tokens
     */
    setMaxTokens(maxTokens) {
        if (maxTokens <= 0) {
            throw new Error("Max tokens must be greater than 0");
        }
        this.maxTokens = maxTokens;
        return this;
    }
    /**
     * Build the chat completion request
     */
    build() {
        if (this.messages.length === 0) {
            throw new Error("At least one message is required");
        }
        return {
            model: this.model,
            messages: this.messages,
            ...(this.temperature !== undefined && { temperature: this.temperature }),
            ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
        };
    }
    /**
     * Execute the chat completion
     */
    async execute(client) {
        const request = this.build();
        return client.chatCompletion(request);
    }
    /**
     * Execute and return just the content
     */
    async getResponse(client) {
        const response = await this.execute(client);
        return response.choices[0]?.message?.content || "";
    }
}
/**
 * Create a new chat builder
 */
export function createChat(model) {
    return new ChatBuilder(model);
}
/**
 * Quick chat function for simple use cases
 */
export async function quickChat(client, model, prompt, options) {
    return client.chat(model, prompt, options);
}
/**
 * Common model constants
 */
export const COMMON_MODELS = {
    GPT_3_5_TURBO: "gpt-3.5-turbo",
    GPT_4: "o4-mini-2025-04-16",
    GPT_4O_MINI: "o4-mini-2025-04-16",
    GPT_4_TURBO: "gpt-4-turbo-preview",
    CLAUDE_3_HAIKU: "claude-3-haiku-20240307",
    CLAUDE_3_SONNET: "claude-3-sonnet-20240229",
    CLAUDE_3_OPUS: "claude-3-opus-20240229",
    LLAMA_2_7B: "llama2:7b",
    LLAMA_2_13B: "llama2:13b",
    LLAMA_2_70B: "llama2:70b",
};
/**
 * Create a conversation with context management
 */
export class Conversation {
    constructor(client, model, options) {
        this.messages = [];
        this.maxContextLength = 4000; // tokens
        this.client = client;
        this.model = model;
        this.systemPrompt = options?.systemPrompt || undefined;
        this.maxContextLength = options?.maxContextLength || 4000;
        this.temperature = options?.temperature || undefined;
        this.maxTokens = options?.maxTokens || undefined;
        if (this.systemPrompt) {
            this.messages.push({ role: "system", content: this.systemPrompt });
        }
    }
    /**
     * Send a message and get response
     */
    async send(message) {
        this.messages.push({ role: "user", content: message });
        // Truncate context if needed (simple implementation)
        await this.truncateContext();
        const request = {
            model: this.model,
            messages: this.messages,
            ...(this.temperature !== undefined && { temperature: this.temperature }),
            ...(this.maxTokens !== undefined && { max_tokens: this.maxTokens }),
        };
        const response = await this.client.chatCompletion(request);
        const assistantMessage = response.choices[0]?.message?.content || "";
        this.messages.push({ role: "assistant", content: assistantMessage });
        return assistantMessage;
    }
    /**
     * Get conversation history
     */
    getHistory() {
        return [...this.messages];
    }
    /**
     * Clear conversation history (keep system prompt)
     */
    clear() {
        this.messages = this.systemPrompt
            ? [{ role: "system", content: this.systemPrompt }]
            : [];
    }
    /**
     * Simple context truncation (remove oldest messages)
     */
    async truncateContext() {
        // This is a simplified implementation
        // In practice, you'd want to count tokens properly
        const maxMessages = Math.floor(this.maxContextLength / 100); // rough estimate
        if (this.messages.length > maxMessages) {
            const systemMessages = this.messages.filter((m) => m.role === "system");
            const otherMessages = this.messages
                .filter((m) => m.role !== "system")
                .slice(-maxMessages + systemMessages.length);
            this.messages = [...systemMessages, ...otherMessages];
        }
    }
}
/**
 * Create a new conversation
 */
export function createConversation(client, model, options) {
    return new Conversation(client, model, options);
}
//# sourceMappingURL=llm.js.map