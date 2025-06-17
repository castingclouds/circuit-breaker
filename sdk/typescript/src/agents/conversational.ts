/**
 * Conversational Agent Implementation for Circuit Breaker SDK
 *
 * Provides advanced conversational AI capabilities with:
 * - Multi-turn conversation management
 * - Context-aware responses
 * - Memory management and persistence
 * - Tool integration and function calling
 * - Conversation summarization and compression
 * - Personality and behavior customization
 *
 * @example
 * ```typescript
 * const agent = new ConversationalAgent({
 *   name: 'Customer Support Bot',
 *   systemPrompt: 'You are a helpful customer support agent.',
 *   memory: { type: 'both', maxSize: 1000 },
 *   tools: [searchKnowledgeBase, createTicket]
 * }, llmRouter);
 *
 * const response = await agent.chat('Hello, I need help with my order.', {
 *   conversationId: 'user-123-conversation',
 *   userId: 'user-123'
 * });
 * ```
 */

import {
  AgentDefinition,
  AgentConfig,
  ChatCompletionRequest,
  ChatMessage,
  Tool,
  ToolCall,
  ChatRole,
} from '../core/types.js';
import {
  AgentError,
  AgentConfigurationError,
  LLMError,
  TimeoutError,
} from '../core/errors.js';
import { Logger, createComponentLogger } from '../utils/logger.js';
import { LLMRouter } from '../llm/router.js';
import { Agent, AgentContext, MemoryManager, ToolBuilderConfig } from './builder.js';

export interface ConversationalConfig {
  /** System prompt that defines agent personality and behavior */
  systemPrompt?: string;

  /** Initial greeting message */
  initialMessage?: string;

  /** Model parameters */
  model?: {
    provider?: string;
    name?: string;
    temperature?: number;
    maxTokens?: number;
    topP?: number;
    frequencyPenalty?: number;
    presencePenalty?: number;
    stopSequences?: string[];
  };

  /** Conversation management */
  conversation?: {
    maxTurns?: number;
    contextWindow?: number;
    turnTimeout?: number; // seconds
    idleTimeout?: number; // seconds
    enableSummarization?: boolean;
    summarizationThreshold?: number; // number of turns
  };

  /** Response generation */
  response?: {
    streaming?: boolean;
    includeSources?: boolean;
    includeReflection?: boolean;
    responseFormat?: 'text' | 'structured' | 'markdown';
    maxRetries?: number;
  };

  /** Personality configuration */
  personality?: {
    tone?: 'professional' | 'friendly' | 'casual' | 'formal' | 'empathetic';
    verbosity?: 'concise' | 'moderate' | 'detailed';
    humor?: boolean;
    creativity?: number; // 0-1
  };

  /** Safety and moderation */
  safety?: {
    enableModeration?: boolean;
    contentFilter?: string[];
    sensitiveTopics?: string[];
    escalationTriggers?: string[];
  };
}

export interface ConversationTurn {
  id: string;
  timestamp: Date;
  userMessage: string;
  agentResponse: string;
  toolCalls?: ToolCall[];
  toolResults?: Record<string, any>;
  metadata: {
    latency: number;
    tokenUsage: {
      prompt: number;
      completion: number;
      total: number;
    };
    model: string;
    cost?: number;
  };
}

export interface ConversationState {
  id: string;
  userId?: string;
  turns: ConversationTurn[];
  summary?: string;
  context: Record<string, any>;
  lastActivity: Date;
  status: 'active' | 'idle' | 'ended';
  metadata: {
    startedAt: Date;
    totalTurns: number;
    totalTokens: number;
    totalCost: number;
  };
}

export interface ConversationMetrics {
  averageResponseTime: number;
  totalConversations: number;
  activeConversations: number;
  averageTurnsPerConversation: number;
  totalTokensUsed: number;
  totalCost: number;
  userSatisfactionScore?: number;
  commonTopics: Record<string, number>;
  errorRate: number;
}

/**
 * Advanced conversational agent with context management
 */
export class ConversationalAgent extends Agent {
  private config: ConversationalConfig;
  private conversations: Map<string, ConversationState> = new Map();
  private metrics: ConversationMetrics;
  private lastActivity: Date = new Date();
  private idleCheckInterval?: NodeJS.Timeout;

  constructor(
    agentDefinition: AgentDefinition,
    config: ConversationalConfig,
    llmRouter: LLMRouter,
    logger?: Logger
  ) {
    super(agentDefinition, llmRouter, logger);

    this.config = {
      model: {
        provider: 'openai',
        name: 'gpt-3.5-turbo',
        temperature: 0.7,
        maxTokens: 1000,
        ...config.model,
      },
      conversation: {
        maxTurns: 50,
        contextWindow: 4000,
        turnTimeout: 60,
        idleTimeout: 1800, // 30 minutes
        enableSummarization: true,
        summarizationThreshold: 10,
        ...config.conversation,
      },
      response: {
        streaming: false,
        includeSources: false,
        includeReflection: false,
        responseFormat: 'text',
        maxRetries: 3,
        ...config.response,
      },
      personality: {
        tone: 'professional',
        verbosity: 'moderate',
        humor: false,
        creativity: 0.7,
        ...config.personality,
      },
      safety: {
        enableModeration: true,
        contentFilter: [],
        sensitiveTopics: [],
        escalationTriggers: [],
        ...config.safety,
      },
      ...config,
    };

    this.metrics = {
      averageResponseTime: 0,
      totalConversations: 0,
      activeConversations: 0,
      averageTurnsPerConversation: 0,
      totalTokensUsed: 0,
      totalCost: 0,
      commonTopics: {},
      errorRate: 0,
    };

    this.startIdleMonitoring();
  }

  /**
   * Process a chat message and generate response
   */
  async chat(message: string, context?: Partial<AgentContext>): Promise<string> {
    const conversationId = context?.conversationId || 'default';
    const userId = context?.userId || 'anonymous';
    const startTime = Date.now();

    try {
      // Get or create conversation
      let conversation = this.conversations.get(conversationId);
      if (!conversation) {
        conversation = await this.createConversation(conversationId, userId);
      }

      // Update activity
      conversation.lastActivity = new Date();
      conversation.status = 'active';

      // Check conversation limits
      if (conversation.turns.length >= this.config.conversation!.maxTurns!) {
        throw new AgentError('Conversation turn limit exceeded');
      }

      // Apply content moderation
      if (this.config.safety?.enableModeration) {
        await this.moderateContent(message, conversation);
      }

      // Prepare conversation history
      const messages = await this.buildConversationHistory(conversation, message);

      // Check context window and summarize if needed
      if (await this.shouldSummarize(conversation)) {
        await this.summarizeConversation(conversation);
      }

      // Generate response
      const result = await this.generateResponse(messages, conversation, context);

      // Create conversation turn
      const turn: ConversationTurn = {
        id: `turn_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        timestamp: new Date(),
        userMessage: message,
        agentResponse: result.response,
        toolCalls: result.toolCalls,
        toolResults: result.toolResults,
        metadata: {
          latency: Date.now() - startTime,
          tokenUsage: result.tokenUsage,
          model: this.config.model!.name!,
          cost: result.cost,
        },
      };

      // Update conversation
      conversation.turns.push(turn);
      conversation.metadata.totalTurns++;
      conversation.metadata.totalTokens += result.tokenUsage.total;
      conversation.metadata.totalCost += result.cost || 0;

      // Update conversation in memory
      await this.memory.updateConversationHistory(conversationId,
        conversation.turns.map(t => [
          { role: 'user' as ChatRole, content: t.userMessage },
          { role: 'assistant' as ChatRole, content: t.agentResponse }
        ]).flat()
      );

      // Update metrics
      this.updateMetrics(turn);

      this.logger.debug('Chat turn completed', {
        conversationId,
        turnId: turn.id,
        latency: turn.metadata.latency,
        tokens: turn.metadata.tokenUsage.total,
      });

      return result.response;
    } catch (error) {
      this.metrics.errorRate = (this.metrics.errorRate + 1) / (this.metrics.totalConversations + 1);
      this.logger.error('Chat failed', { error, conversationId, message });
      throw new AgentError(`Chat failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  /**
   * Start a new conversation with optional initial message
   */
  async startConversation(
    conversationId: string,
    userId?: string,
    initialContext?: Record<string, any>
  ): Promise<string> {
    const conversation = await this.createConversation(conversationId, userId, initialContext);

    if (this.config.initialMessage) {
      return this.config.initialMessage;
    }

    return 'Hello! How can I assist you today?';
  }

  /**
   * End a conversation
   */
  async endConversation(conversationId: string): Promise<void> {
    const conversation = this.conversations.get(conversationId);
    if (conversation) {
      conversation.status = 'ended';
      conversation.lastActivity = new Date();

      // Generate final summary if enabled
      if (this.config.conversation?.enableSummarization && conversation.turns.length > 0) {
        await this.summarizeConversation(conversation);
      }

      this.logger.info('Conversation ended', {
        conversationId,
        turns: conversation.turns.length,
        duration: Date.now() - conversation.metadata.startedAt.getTime(),
      });
    }
  }

  /**
   * Get conversation history
   */
  getConversation(conversationId: string): ConversationState | undefined {
    return this.conversations.get(conversationId);
  }

  /**
   * Get all active conversations
   */
  getActiveConversations(): ConversationState[] {
    return Array.from(this.conversations.values()).filter(c => c.status === 'active');
  }

  /**
   * Get conversation metrics
   */
  getMetrics(): ConversationMetrics {
    return { ...this.metrics };
  }

  /**
   * Clear conversation history
   */
  async clearConversation(conversationId: string): Promise<void> {
    const conversation = this.conversations.get(conversationId);
    if (conversation) {
      conversation.turns = [];
      conversation.summary = undefined;
      conversation.context = {};
      await this.memory.store(`conversation:${conversationId}`, [], 'short_term');
    }
  }

  /**
   * Export conversation data
   */
  exportConversations(): Record<string, ConversationState> {
    const exported: Record<string, ConversationState> = {};
    for (const [id, conversation] of this.conversations) {
      exported[id] = { ...conversation };
    }
    return exported;
  }

  /**
   * Import conversation data
   */
  importConversations(data: Record<string, ConversationState>): void {
    for (const [id, conversation] of Object.entries(data)) {
      this.conversations.set(id, conversation);
    }
  }

  private async createConversation(
    conversationId: string,
    userId?: string,
    initialContext?: Record<string, any>
  ): Promise<ConversationState> {
    const conversation: ConversationState = {
      id: conversationId,
      userId,
      turns: [],
      context: initialContext || {},
      lastActivity: new Date(),
      status: 'active',
      metadata: {
        startedAt: new Date(),
        totalTurns: 0,
        totalTokens: 0,
        totalCost: 0,
      },
    };

    this.conversations.set(conversationId, conversation);
    this.metrics.totalConversations++;
    this.metrics.activeConversations++;

    this.logger.info('New conversation created', { conversationId, userId });
    return conversation;
  }

  private async buildConversationHistory(
    conversation: ConversationState,
    newMessage: string
  ): Promise<ChatMessage[]> {
    const messages: ChatMessage[] = [];

    // Add system prompt
    if (this.config.systemPrompt) {
      messages.push({
        role: 'system',
        content: this.enhanceSystemPrompt(this.config.systemPrompt, conversation),
      });
    }

    // Add conversation summary if exists
    if (conversation.summary) {
      messages.push({
        role: 'system',
        content: `Previous conversation summary: ${conversation.summary}`,
      });
    }

    // Add recent conversation turns
    const recentTurns = conversation.turns.slice(-5); // Keep last 5 turns for context
    for (const turn of recentTurns) {
      messages.push({ role: 'user', content: turn.userMessage });
      messages.push({ role: 'assistant', content: turn.agentResponse });
    }

    // Add new user message
    messages.push({ role: 'user', content: newMessage });

    return messages;
  }

  private enhanceSystemPrompt(basePrompt: string, conversation: ConversationState): string {
    let enhancedPrompt = basePrompt;

    // Add personality traits
    const personality = this.config.personality!;
    enhancedPrompt += `\n\nPersonality: Communicate with a ${personality.tone} tone, be ${personality.verbosity} in your responses.`;

    if (personality.humor) {
      enhancedPrompt += ' Feel free to use appropriate humor when suitable.';
    }

    // Add conversation context
    if (Object.keys(conversation.context).length > 0) {
      enhancedPrompt += `\n\nConversation context: ${JSON.stringify(conversation.context)}`;
    }

    // Add safety guidelines
    if (this.config.safety?.contentFilter?.length) {
      enhancedPrompt += `\n\nAvoid discussing: ${this.config.safety.contentFilter.join(', ')}`;
    }

    return enhancedPrompt;
  }

  private async generateResponse(
    messages: ChatMessage[],
    conversation: ConversationState,
    context?: Partial<AgentContext>
  ): Promise<{
    response: string;
    toolCalls?: ToolCall[];
    toolResults?: Record<string, any>;
    tokenUsage: { prompt: number; completion: number; total: number };
    cost?: number;
  }> {
    const request: ChatCompletionRequest = {
      model: this.config.model!.name!,
      messages,
      temperature: this.config.model!.temperature,
      max_tokens: this.config.model!.maxTokens,
      top_p: this.config.model!.topP,
      frequency_penalty: this.config.model!.frequencyPenalty,
      presence_penalty: this.config.model!.presencePenalty,
      stop: this.config.model!.stopSequences,
      tools: this.getToolsForLLM(),
      tool_choice: this.tools.size > 0 ? 'auto' : undefined,
    };

    // Apply timeout
    const timeout = this.config.conversation!.turnTimeout! * 1000;
    const timeoutPromise = new Promise((_, reject) => {
      setTimeout(() => reject(new TimeoutError('Response generation timeout')), timeout);
    });

    const responsePromise = this.llmRouter.chatCompletion(request);
    const response = await Promise.race([responsePromise, timeoutPromise]);

    if (!(response && typeof response === 'object' && 'choices' in response)) {
      throw new AgentError('Invalid response from LLM');
    }

    const assistantMessage = response.choices[0].message;
    let finalResponse = assistantMessage.content || '';
    let toolCalls: ToolCall[] | undefined;
    let toolResults: Record<string, any> | undefined;

    // Handle tool calls
    if (assistantMessage.tool_calls) {
      toolCalls = assistantMessage.tool_calls;
      toolResults = {};

      const agentContext: AgentContext = {
        agent: this,
        conversationId: conversation.id,
        userId: conversation.userId,
        session: context?.session || {},
        memory: this.memory,
        llm: this.llmRouter,
        logger: this.logger,
      };

      // Execute tools
      for (const toolCall of toolCalls) {
        try {
          const result = await this.executeTool(toolCall, agentContext);
          toolResults[toolCall.id] = result;
        } catch (error) {
          toolResults[toolCall.id] = { error: String(error) };
        }
      }

      // Generate follow-up response with tool results
      const followUpMessages: ChatMessage[] = [
        ...messages,
        { role: 'assistant', content: '', tool_calls: toolCalls },
        ...toolCalls.map(tc => ({
          role: 'tool' as ChatRole,
          content: JSON.stringify(toolResults[tc.id]),
          tool_call_id: tc.id,
        })),
      ];

      const followUpRequest: ChatCompletionRequest = {
        ...request,
        messages: followUpMessages,
      };

      const followUpResponse = await this.llmRouter.chatCompletion(followUpRequest);
      finalResponse = followUpResponse.choices[0].message.content || finalResponse;
    }

    return {
      response: finalResponse,
      toolCalls,
      toolResults,
      tokenUsage: {
        prompt: response.usage?.prompt_tokens || 0,
        completion: response.usage?.completion_tokens || 0,
        total: response.usage?.total_tokens || 0,
      },
      cost: this.calculateCost(response.usage?.total_tokens || 0),
    };
  }

  private async shouldSummarize(conversation: ConversationState): boolean {
    if (!this.config.conversation?.enableSummarization) return false;

    const threshold = this.config.conversation?.summarizationThreshold || 10;
    return conversation.turns.length >= threshold && !conversation.summary;
  }

  private async summarizeConversation(conversation: ConversationState): Promise<void> {
    if (conversation.turns.length === 0) return;

    try {
      const conversationText = conversation.turns
        .map(turn => `User: ${turn.userMessage}\nAssistant: ${turn.agentResponse}`)
        .join('\n\n');

      const summaryRequest: ChatCompletionRequest = {
        model: this.config.model!.name!,
        messages: [
          {
            role: 'system',
            content: 'Summarize the following conversation concisely, capturing key points and outcomes:',
          },
          { role: 'user', content: conversationText },
        ],
        temperature: 0.3,
        max_tokens: 200,
      };

      const summaryResponse = await this.llmRouter.chatCompletion(summaryRequest);
      conversation.summary = summaryResponse.choices[0].message.content || '';

      this.logger.debug('Conversation summarized', {
        conversationId: conversation.id,
        originalTurns: conversation.turns.length,
        summaryLength: conversation.summary.length,
      });
    } catch (error) {
      this.logger.warn('Failed to summarize conversation', { error, conversationId: conversation.id });
    }
  }

  private async moderateContent(message: string, conversation: ConversationState): Promise<void> {
    const safety = this.config.safety!;

    // Check content filters
    if (safety.contentFilter?.length) {
      const lowercaseMessage = message.toLowerCase();
      for (const filter of safety.contentFilter) {
        if (lowercaseMessage.includes(filter.toLowerCase())) {
          throw new AgentError(`Message contains filtered content: ${filter}`);
        }
      }
    }

    // Check escalation triggers
    if (safety.escalationTriggers?.length) {
      const lowercaseMessage = message.toLowerCase();
      for (const trigger of safety.escalationTriggers) {
        if (lowercaseMessage.includes(trigger.toLowerCase())) {
          this.logger.warn('Escalation trigger detected', {
            conversationId: conversation.id,
            trigger,
            message,
          });
          // Could trigger notifications or escalation workflows here
        }
      }
    }
  }

  private calculateCost(tokens: number): number {
    // Simple cost calculation - would be enhanced with actual pricing
    const costPerToken = 0.000001; // $0.000001 per token
    return tokens * costPerToken;
  }

  private updateMetrics(turn: ConversationTurn): void {
    // Update response time
    this.metrics.averageResponseTime =
      (this.metrics.averageResponseTime + turn.metadata.latency) / 2;

    // Update token usage
    this.metrics.totalTokensUsed += turn.metadata.tokenUsage.total;

    // Update cost
    this.metrics.totalCost += turn.metadata.cost || 0;

    // Update conversation metrics
    const activeConversations = this.getActiveConversations().length;
    this.metrics.activeConversations = activeConversations;

    if (this.metrics.totalConversations > 0) {
      const totalTurns = Array.from(this.conversations.values())
        .reduce((sum, conv) => sum + conv.turns.length, 0);
      this.metrics.averageTurnsPerConversation = totalTurns / this.metrics.totalConversations;
    }
  }

  private startIdleMonitoring(): void {
    // Check for idle conversations every 5 minutes
    this.idleCheckInterval = setInterval(() => {
      const now = new Date();
      const idleTimeout = this.config.conversation!.idleTimeout! * 1000;

      for (const [id, conversation] of this.conversations) {
        if (conversation.status === 'active') {
          const idleTime = now.getTime() - conversation.lastActivity.getTime();
          if (idleTime > idleTimeout) {
            conversation.status = 'idle';
            this.metrics.activeConversations--;
            this.logger.debug('Conversation marked as idle', { conversationId: id, idleTime });
          }
        }
      }
    }, 5 * 60 * 1000); // 5 minutes
  }

  /**
   * Get conversation state for serialization
   */
  getState(): {
    conversations: Record<string, ConversationState>;
    metrics: ConversationMetrics;
  } {
    const conversations: Record<string, ConversationState> = {};
    for (const [id, conv] of this.conversations) {
      conversations[id] = conv;
    }

    return {
      conversations,
      metrics: this.metrics,
    };
  }

  /**
   * Set conversation state from serialization
   */
  async setState(state: {
    conversations: Record<string, ConversationState>;
    metrics: ConversationMetrics;
  }): Promise<void> {
    this.conversations.clear();
    for (const [id, conv] of Object.entries(state.conversations)) {
      this.conversations.set(id, conv);
    }
    this.metrics = state.metrics;
  }

  /**
   * Clean up resources
   */
  async destroy(): Promise<void> {
    if (this.idleCheckInterval) {
      clearInterval(this.idleCheckInterval);
    }

    // End all active conversations
    for (const [id, conversation] of this.conversations) {
      if (conversation.status === 'active') {
        await this.endConversation(id);
      }
    }

    await super.destroy();
  }
}

/**
 * Factory function for creating conversational agents
 */
export function createConversationalAgent(
  config: ConversationalConfig & { name: string; description?: string },
  llmRouter: LLMRouter,
  logger?: Logger
): ConversationalAgent {
  const agentDefinition: AgentDefinition = {
    id: `conv_agent_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    name: config.name,
    type: 'conversational',
    description: config.description,
    config: {
      llmProvider: config.model?.provider || 'openai',
      memory: { type: 'both', maxSize: 1000 },
    },
    metadata: {
      createdAt: new Date().toISOString(),
      version: '1.0.0',
    },
  };

  return new ConversationalAgent(agentDefinition, config, llmRouter, logger);
}

/**
 * Common conversational agent templates
 */
export const ConversationalTemplates = {
  customerSupport: (llmRouter: LLMRouter): ConversationalAgent => {
    return createConversationalAgent({
      name: 'Customer Support Agent',
      description: 'AI-powered customer support assistant',
      systemPrompt: `You are a professional customer support agent. Your goal is to help customers with their inquiries, resolve issues, and provide excellent service.

Guidelines:
- Be polite, empathetic, and professional
- Listen actively to customer concerns
- Ask clarifying questions when needed
- Provide clear, step-by-step solutions
- Escalate complex issues when appropriate
- Always ensure customer satisfaction`,
      personality: {
        tone: 'empathetic',
        verbosity: 'moderate',
        humor: false,
        creativity: 0.3,
      },
      conversation: {
        maxTurns: 30,
        enableSummarization: true,
        summarizationThreshold: 8,
      },
      safety: {
        enableModeration: true,
        escalationTriggers: ['angry', 'frustrated', 'cancel', 'refund', 'legal'],
      },
    }, llmRouter);
  },

  salesAssistant: (llmRouter: LLMRouter): ConversationalAgent => {
    return createConversationalAgent({
      name: 'Sales Assistant',
      description: 'AI-powered sales and product advisor',
      systemPrompt: `You are a knowledgeable sales assistant. Help customers find the right products, answer questions, and guide them through the purchase process.

Guidelines:
- Understand customer needs and preferences
- Provide accurate product information
- Make relevant recommendations
- Be consultative, not pushy
- Focus on customer value
- Build rapport and trust`,
      personality: {
        tone: 'friendly',
        verbosity: 'detailed',
        humor: true,
        creativity: 0.6,
      },
      conversation: {
        maxTurns: 40,
        enableSummarization: true,
      },
    }, llmRouter);
  },

  technicalSupport: (llmRouter: LLMRouter): ConversationalAgent => {
    return createConversationalAgent({
      name: 'Technical Support Specialist',
      description: 'AI-powered technical support assistant',
      systemPrompt: `You are a technical support specialist. Help users troubleshoot technical issues and provide step-by-step guidance.

Guidelines:
- Ask specific diagnostic questions
- Provide clear, step-by-step instructions
- Use simple language, avoid jargon
- Be patient and thorough
- Verify solutions work
- Document resolutions`,
      personality: {
        tone: 'professional',
        verbosity: 'detailed',
        humor: false,
        creativity: 0.2,
      },
      conversation: {
        maxTurns: 25,
        enableSummarization: true,
        summarizationThreshold: 6,
      },
    }, llmRouter);
  },
};
