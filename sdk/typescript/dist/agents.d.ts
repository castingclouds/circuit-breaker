/**
 * Agents API client for Circuit Breaker TypeScript SDK
 * Uses GraphQL for all operations
 */
import { Agent, AgentCreateInput, AgentType, ChatMessage, PaginationOptions } from "./types";
import { Client } from "./client";
export declare class AgentClient {
    private client;
    constructor(client: Client);
    /**
     * Create a new agent
     */
    create(input: AgentCreateInput): Promise<Agent>;
    /**
     * Get an agent by ID
     */
    get(id: string): Promise<Agent>;
    /**
     * List all agents
     */
    list(_options?: PaginationOptions): Promise<Agent[]>;
    /**
     * Update an existing agent
     */
    update(id: string, updates: Partial<AgentCreateInput>): Promise<Agent>;
    /**
     * Delete an agent
     */
    delete(id: string): Promise<boolean>;
    /**
     * Chat with an agent
     */
    chat(id: string, messages: ChatMessage[]): Promise<any>;
    /**
     * Execute an agent with input
     */
    execute(id: string, input: Record<string, any>): Promise<any>;
}
export declare class AgentBuilder {
    private agent;
    /**
     * Set agent name
     */
    setName(name: string): AgentBuilder;
    /**
     * Set agent description
     */
    setDescription(description: string): AgentBuilder;
    /**
     * Set agent type
     */
    setType(type: AgentType): AgentBuilder;
    /**
     * Set LLM provider
     */
    setLLMProvider(provider: string): AgentBuilder;
    /**
     * Set LLM model
     */
    setModel(model: string): AgentBuilder;
    /**
     * Set temperature
     */
    setTemperature(temperature: number): AgentBuilder;
    /**
     * Set max tokens
     */
    setMaxTokens(maxTokens: number): AgentBuilder;
    /**
     * Set system prompt
     */
    setSystemPrompt(prompt: string): AgentBuilder;
    /**
     * Add a tool
     */
    addTool(name: string, description: string, parameters: Record<string, any>): AgentBuilder;
    /**
     * Set memory configuration
     */
    setMemory(type: "short_term" | "long_term" | "persistent", options?: {
        max_entries?: number;
        ttl?: number;
    }): AgentBuilder;
    /**
     * Build the agent definition
     */
    build(): AgentCreateInput;
}
/**
 * Create a new agent builder
 */
export declare function createAgent(name: string): AgentBuilder;
//# sourceMappingURL=agents.d.ts.map