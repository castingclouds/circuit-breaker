/**
 * Agent Builder for Circuit Breaker SDK
 *
 * Provides a fluent API for creating conversational and state machine agents with:
 * - Conversational AI agents with memory and tool integration
 * - State machine agents for complex workflows
 * - Integration with LLM Router for multi-provider AI
 * - Memory management and persistence
 * - Tool/function calling capabilities
 * - Workflow integration
 *
 * @example
 * ```typescript
 * const agent = new AgentBuilder()
 *   .setName('Customer Support Agent')
 *   .setType('conversational')
 *   .setSystemPrompt('You are a helpful customer support agent.')
 *   .addTool('search_knowledge_base', searchKnowledgeBase)
 *   .addTool('create_ticket', createTicket)
 *   .enableMemory({ type: 'both', maxSize: 1000 })
 *   .setLLMProvider('openai-gpt4')
 *   .build();
 *
 * const response = await agent.chat('Hello, I need help with my order.');
 * ```
 */

import {
  AgentDefinition,
  AgentType,
  AgentConfig,
  MemoryConfig,
  StateMachineConfig,
  AgentState,
  StateTransition,
  StateAction,
  ChatCompletionRequest,
  ChatMessage,
  Tool,
  ToolCall,
} from "../core/types.js";
import {
  AgentError,
  AgentNotFoundError,
  AgentConfigurationError,
  LLMError,
  ValidationError,
} from "../core/errors.js";
import { Logger, createComponentLogger } from "../utils/logger.js";
import { LLMRouter } from "../llm/router.js";

export interface AgentBuilderConfig {
  /** Agent name */
  name?: string;

  /** Agent description */
  description?: string;

  /** Default LLM provider */
  defaultLLMProvider?: string;

  /** Enable debug logging */
  debug?: boolean;

  /** Custom logger instance */
  logger?: Logger;
}

export interface ConversationalAgentConfig {
  /** System prompt */
  systemPrompt?: string;

  /** Initial message */
  initialMessage?: string;

  /** Available tools */
  tools?: Tool[];

  /** Tool choice strategy */
  toolChoice?: "auto" | "required" | "none" | { function: { name: string } };

  /** Conversation settings */
  conversation?: {
    maxTurns?: number;
    temperature?: number;
    maxTokens?: number;
    stopSequences?: string[];
  };

  /** Context window management */
  contextWindow?: {
    maxTokens?: number;
    truncationStrategy?: "oldest" | "summarize" | "sliding";
    preserveSystemPrompt?: boolean;
  };
}

export interface StateMachineAgentConfig {
  /** Initial state */
  initialState: string;

  /** Available states */
  states: Record<string, AgentState>;

  /** State transitions */
  transitions: StateTransition[];

  /** Global variables */
  variables?: Record<string, any>;

  /** State timeout settings */
  timeouts?: Record<string, number>;
}

export interface MemoryBuilderConfig {
  /** Memory type */
  type: "short_term" | "long_term" | "both";

  /** Maximum memory size */
  maxSize?: number;

  /** Enable persistence */
  persistent?: boolean;

  /** Memory backend */
  backend?: "memory" | "redis" | "database";

  /** Memory retention settings */
  retention?: {
    shortTermTTL?: number; // seconds
    longTermTTL?: number; // seconds
    compressionThreshold?: number;
  };
}

export interface ToolBuilderConfig {
  /** Tool name */
  name: string;

  /** Tool description */
  description: string;

  /** Tool parameters schema */
  parameters: Record<string, any>;

  /** Tool implementation */
  implementation: ToolImplementation;

  /** Tool configuration */
  config?: {
    timeout?: number;
    retries?: number;
    async?: boolean;
  };
}

export type ToolImplementation = (
  parameters: Record<string, any>,
  context: AgentContext,
) => Promise<any> | any;

export interface AgentContext {
  /** Agent instance */
  agent: Agent;

  /** Current conversation ID */
  conversationId?: string;

  /** User ID */
  userId?: string;

  /** Session data */
  session: Record<string, any>;

  /** Memory access */
  memory: MemoryManager;

  /** LLM router access */
  llm: LLMRouter;

  /** Logger instance */
  logger: Logger;
}

export interface AgentBuilderResult {
  agent: Agent;
  config: AgentDefinition;
  validation: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  };
}

/**
 * Memory manager for agent conversations
 */
export class MemoryManager {
  private shortTermMemory: Map<string, any> = new Map();
  private longTermMemory: Map<string, any> = new Map();
  private config: MemoryBuilderConfig;
  private logger: Logger;

  constructor(config: MemoryBuilderConfig, logger?: Logger) {
    this.config = config;
    this.logger = logger || createComponentLogger("MemoryManager");
  }

  /**
   * Store data in memory
   */
  async store(
    key: string,
    value: any,
    type: "short_term" | "long_term" = "short_term",
  ): Promise<void> {
    const memory =
      type === "short_term" ? this.shortTermMemory : this.longTermMemory;

    // Check size limits
    if (this.config.maxSize && memory.size >= this.config.maxSize) {
      this.evictOldest(memory);
    }

    memory.set(key, {
      value,
      timestamp: Date.now(),
      type,
    });

    this.logger.debug(`Stored memory: ${key} (${type})`, { key, type });
  }

  /**
   * Retrieve data from memory
   */
  async retrieve(key: string, type?: "short_term" | "long_term"): Promise<any> {
    if (type) {
      const memory =
        type === "short_term" ? this.shortTermMemory : this.longTermMemory;
      const item = memory.get(key);
      return item?.value;
    }

    // Search both memories
    const shortTerm = this.shortTermMemory.get(key);
    if (shortTerm) return shortTerm.value;

    const longTerm = this.longTermMemory.get(key);
    return longTerm?.value;
  }

  /**
   * Get conversation history
   */
  async getConversationHistory(conversationId: string): Promise<ChatMessage[]> {
    const history = await this.retrieve(
      `conversation:${conversationId}`,
      "short_term",
    );
    return history || [];
  }

  /**
   * Update conversation history
   */
  async updateConversationHistory(
    conversationId: string,
    messages: ChatMessage[],
  ): Promise<void> {
    await this.store(`conversation:${conversationId}`, messages, "short_term");
  }

  /**
   * Clear memory
   */
  async clear(type?: "short_term" | "long_term"): Promise<void> {
    if (type) {
      const memory =
        type === "short_term" ? this.shortTermMemory : this.longTermMemory;
      memory.clear();
    } else {
      this.shortTermMemory.clear();
      this.longTermMemory.clear();
    }
  }

  /**
   * Get memory statistics
   */
  getStats(): {
    shortTerm: { size: number; maxSize: number };
    longTerm: { size: number; maxSize: number };
  } {
    return {
      shortTerm: {
        size: this.shortTermMemory.size,
        maxSize: this.config.maxSize || 0,
      },
      longTerm: {
        size: this.longTermMemory.size,
        maxSize: this.config.maxSize || 0,
      },
    };
  }

  private evictOldest(memory: Map<string, any>): void {
    let oldestKey: string | null = null;
    let oldestTimestamp = Infinity;

    for (const [key, item] of memory.entries()) {
      if (item.timestamp < oldestTimestamp) {
        oldestTimestamp = item.timestamp;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      memory.delete(oldestKey);
      this.logger.debug(`Evicted oldest memory: ${oldestKey}`);
    }
  }
}

/**
 * Base agent class
 */
export abstract class Agent {
  public readonly id: string;
  public readonly name: string;
  public readonly type: AgentType;
  protected config: AgentDefinition;
  protected memory: MemoryManager;
  protected llmRouter: LLMRouter;
  protected logger: Logger;
  protected tools: Map<string, ToolBuilderConfig> = new Map();

  constructor(config: AgentDefinition, llmRouter: LLMRouter, logger?: Logger) {
    this.id =
      config.id ||
      `agent_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    this.name = config.name;
    this.type = config.type;
    this.config = config;
    this.llmRouter = llmRouter;
    this.logger = logger || createComponentLogger(`Agent:${this.name}`);

    // Initialize memory
    this.memory = new MemoryManager(
      config.config?.memory || { type: "short_term" },
      this.logger,
    );
  }

  abstract chat(
    message: string,
    context?: Partial<AgentContext>,
  ): Promise<string>;
  abstract getState?(): any;
  abstract setState?(state: any): Promise<void>;

  /**
   * Add a tool to the agent
   */
  addTool(tool: ToolBuilderConfig): void {
    this.tools.set(tool.name, tool);
    this.logger.debug(`Added tool: ${tool.name}`);
  }

  /**
   * Execute a tool
   */
  async executeTool(toolCall: ToolCall, context: AgentContext): Promise<any> {
    const tool = this.tools.get(toolCall.function.name);
    if (!tool) {
      throw new AgentError(`Tool not found: ${toolCall.function.name}`);
    }

    try {
      const parameters = JSON.parse(toolCall.function.arguments);
      const result = await tool.implementation(parameters, context);

      this.logger.debug(`Executed tool: ${tool.name}`, { parameters, result });
      return result;
    } catch (error) {
      this.logger.error(`Tool execution failed: ${tool.name}`, { error });
      throw new AgentError(`Tool execution failed: ${error}`);
    }
  }

  /**
   * Get available tools as LLM tools format
   */
  getToolsForLLM(): Tool[] {
    return Array.from(this.tools.values()).map((tool) => ({
      type: "function",
      function: {
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
      },
    }));
  }

  /**
   * Get agent statistics
   */
  getStats(): {
    id: string;
    name: string;
    type: AgentType;
    toolCount: number;
    memoryStats: any;
  } {
    return {
      id: this.id,
      name: this.name,
      type: this.type,
      toolCount: this.tools.size,
      memoryStats: this.memory.getStats(),
    };
  }

  /**
   * Clean up agent resources
   */
  async destroy(): Promise<void> {
    await this.memory.clear();
    this.tools.clear();
    this.logger.info(`Agent ${this.name} destroyed`);
  }
}

/**
 * Conversational agent implementation
 */
export class ConversationalAgent extends Agent {
  private conversationalConfig: ConversationalAgentConfig;
  private activeConversations: Map<string, ChatMessage[]> = new Map();

  constructor(
    config: AgentDefinition,
    conversationalConfig: ConversationalAgentConfig,
    llmRouter: LLMRouter,
    logger?: Logger,
  ) {
    super(config, llmRouter, logger);
    this.conversationalConfig = conversationalConfig;
  }

  async chat(
    message: string,
    context?: Partial<AgentContext>,
  ): Promise<string> {
    const conversationId = context?.conversationId || "default";
    const userId = context?.userId || "anonymous";

    try {
      // Get conversation history
      let messages = await this.memory.getConversationHistory(conversationId);

      // Initialize with system prompt if new conversation
      if (messages.length === 0 && this.conversationalConfig.systemPrompt) {
        messages.push({
          role: "system",
          content: this.conversationalConfig.systemPrompt,
        });
      }

      // Add user message
      messages.push({
        role: "user",
        content: message,
      });

      // Prepare LLM request
      const request: ChatCompletionRequest = {
        model: this.config.config?.llmProvider || "gpt-3.5-turbo",
        messages,
        temperature: this.conversationalConfig.conversation?.temperature || 0.7,
        max_tokens: this.conversationalConfig.conversation?.maxTokens || 1000,
        tools: this.getToolsForLLM(),
        tool_choice: this.conversationalConfig.toolChoice || "auto",
      };

      // Make LLM request
      const response = await this.llmRouter.chatCompletion(request);
      const assistantMessage = response.choices[0].message;

      // Handle tool calls
      if (assistantMessage.tool_calls) {
        const agentContext: AgentContext = {
          agent: this,
          conversationId,
          userId,
          session: context?.session || {},
          memory: this.memory,
          llm: this.llmRouter,
          logger: this.logger,
        };

        // Execute tool calls
        for (const toolCall of assistantMessage.tool_calls) {
          try {
            const toolResult = await this.executeTool(toolCall, agentContext);

            // Add tool call and result to conversation
            messages.push({
              role: "assistant",
              content: "",
              tool_calls: [toolCall],
            });

            messages.push({
              role: "tool",
              content: JSON.stringify(toolResult),
              tool_call_id: toolCall.id,
            });
          } catch (error) {
            this.logger.error(`Tool call failed: ${toolCall.function.name}`, {
              error,
            });

            messages.push({
              role: "tool",
              content: `Error: ${error}`,
              tool_call_id: toolCall.id,
            });
          }
        }

        // Make another LLM request with tool results
        const followUpRequest: ChatCompletionRequest = {
          ...request,
          messages,
        };

        const followUpResponse =
          await this.llmRouter.chatCompletion(followUpRequest);
        messages.push(followUpResponse.choices[0].message);
      } else {
        // Add assistant response
        messages.push(assistantMessage);
      }

      // Update conversation history
      await this.memory.updateConversationHistory(conversationId, messages);

      // Store in active conversations
      this.activeConversations.set(conversationId, messages);

      return assistantMessage.content || "";
    } catch (error) {
      this.logger.error("Chat failed", { error, conversationId, userId });
      throw new AgentError(`Chat failed: ${error}`);
    }
  }

  /**
   * Get conversation state
   */
  getState(): { conversations: Record<string, ChatMessage[]> } {
    const conversations: Record<string, ChatMessage[]> = {};
    for (const [id, messages] of this.activeConversations) {
      conversations[id] = messages;
    }
    return { conversations };
  }

  /**
   * Set conversation state
   */
  async setState(state: {
    conversations: Record<string, ChatMessage[]>;
  }): Promise<void> {
    this.activeConversations.clear();
    for (const [id, messages] of Object.entries(state.conversations)) {
      this.activeConversations.set(id, messages);
      await this.memory.updateConversationHistory(id, messages);
    }
  }

  /**
   * Clear conversation
   */
  async clearConversation(conversationId: string): Promise<void> {
    this.activeConversations.delete(conversationId);
    await this.memory.store(`conversation:${conversationId}`, [], "short_term");
  }
}

/**
 * State machine agent implementation
 */
export class StateMachineAgent extends Agent {
  private stateMachineConfig: StateMachineAgentConfig;
  private currentState: string;
  private variables: Record<string, any> = {};
  private stateHistory: string[] = [];

  constructor(
    config: AgentDefinition,
    stateMachineConfig: StateMachineAgentConfig,
    llmRouter: LLMRouter,
    logger?: Logger,
  ) {
    super(config, llmRouter, logger);
    this.stateMachineConfig = stateMachineConfig;
    this.currentState = stateMachineConfig.initialState;
    this.variables = { ...stateMachineConfig.variables };
  }

  async chat(
    message: string,
    context?: Partial<AgentContext>,
  ): Promise<string> {
    try {
      // Get current state configuration
      const state = this.stateMachineConfig.states[this.currentState];
      if (!state) {
        throw new AgentError(`Invalid state: ${this.currentState}`);
      }

      // Execute state entry actions
      if (state.onEntry) {
        await this.executeActions(state.onEntry, context);
      }

      // Process message with LLM if state has prompt
      let response = "";
      if (state.prompt) {
        const systemPrompt = this.buildStatePrompt(state, message);

        const request: ChatCompletionRequest = {
          model: this.config.config?.llmProvider || "gpt-3.5-turbo",
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: message },
          ],
          temperature: 0.3,
          max_tokens: 500,
        };

        const llmResponse = await this.llmRouter.chatCompletion(request);
        response = llmResponse.choices[0].message.content || "";
      }

      // Check for state transitions
      const transition = this.findTransition(message, response);
      if (transition) {
        await this.transitionToState(transition.toState, context);
      }

      return (
        response || state.defaultResponse || "State processed successfully."
      );
    } catch (error) {
      this.logger.error("State machine chat failed", {
        error,
        currentState: this.currentState,
      });
      throw new AgentError(`State machine chat failed: ${error}`);
    }
  }

  /**
   * Get current state
   */
  getState(): {
    currentState: string;
    variables: Record<string, any>;
    history: string[];
  } {
    return {
      currentState: this.currentState,
      variables: { ...this.variables },
      history: [...this.stateHistory],
    };
  }

  /**
   * Set state
   */
  async setState(state: {
    currentState: string;
    variables: Record<string, any>;
    history: string[];
  }): Promise<void> {
    this.currentState = state.currentState;
    this.variables = { ...state.variables };
    this.stateHistory = [...state.history];
  }

  /**
   * Transition to a new state
   */
  async transitionToState(
    newState: string,
    context?: Partial<AgentContext>,
  ): Promise<void> {
    const currentState = this.stateMachineConfig.states[this.currentState];
    const targetState = this.stateMachineConfig.states[newState];

    if (!targetState) {
      throw new AgentError(`Target state not found: ${newState}`);
    }

    this.logger.debug(`State transition: ${this.currentState} -> ${newState}`);

    // Execute exit actions
    if (currentState?.onExit) {
      await this.executeActions(currentState.onExit, context);
    }

    // Update state
    this.stateHistory.push(this.currentState);
    this.currentState = newState;

    // Execute entry actions
    if (targetState.onEntry) {
      await this.executeActions(targetState.onEntry, context);
    }
  }

  private buildStatePrompt(state: AgentState, userMessage: string): string {
    let prompt = state.prompt || "";

    // Replace variables in prompt
    for (const [key, value] of Object.entries(this.variables)) {
      prompt = prompt.replace(new RegExp(`{{${key}}}`, "g"), String(value));
    }

    // Add context information
    prompt += `\n\nCurrent state: ${this.currentState}`;
    prompt += `\nUser message: ${userMessage}`;

    if (state.availableTransitions) {
      prompt += `\nAvailable transitions: ${state.availableTransitions.join(", ")}`;
    }

    return prompt;
  }

  private findTransition(
    userMessage: string,
    response: string,
  ): StateTransition | null {
    for (const transition of this.stateMachineConfig.transitions) {
      if (transition.fromState !== this.currentState) continue;

      // Check transition conditions
      if (transition.condition) {
        const conditionMet = this.evaluateCondition(
          transition.condition,
          userMessage,
          response,
        );
        if (conditionMet) {
          return transition;
        }
      }

      // Check trigger patterns
      if (
        transition.trigger &&
        this.matchesTrigger(transition.trigger, userMessage)
      ) {
        return transition;
      }
    }

    return null;
  }

  private evaluateCondition(
    condition: string,
    userMessage: string,
    response: string,
  ): boolean {
    try {
      // Simple condition evaluation (can be enhanced)
      const context = {
        userMessage,
        response,
        variables: this.variables,
        currentState: this.currentState,
      };

      // Replace variables in condition
      let evaluableCondition = condition;
      for (const [key, value] of Object.entries(context)) {
        evaluableCondition = evaluableCondition.replace(
          new RegExp(`\\b${key}\\b`, "g"),
          JSON.stringify(value),
        );
      }

      // Basic condition evaluation (should use a safer evaluator in production)
      return eval(evaluableCondition);
    } catch (error) {
      this.logger.warn(`Condition evaluation failed: ${condition}`, { error });
      return false;
    }
  }

  private matchesTrigger(trigger: string, userMessage: string): boolean {
    // Simple pattern matching (can be enhanced with regex)
    return userMessage.toLowerCase().includes(trigger.toLowerCase());
  }

  private async executeActions(
    actions: StateAction[],
    context?: Partial<AgentContext>,
  ): Promise<void> {
    for (const action of actions) {
      try {
        switch (action.type) {
          case "function_call":
            if (action.config.functionName) {
              // Execute function/tool
              const tool = this.tools.get(action.config.functionName);
              if (tool) {
                const agentContext: AgentContext = {
                  agent: this,
                  conversationId: context?.conversationId || "default",
                  userId: context?.userId || "anonymous",
                  session: context?.session || {},
                  memory: this.memory,
                  llm: this.llmRouter,
                  logger: this.logger,
                };

                await tool.implementation(
                  action.config.parameters || {},
                  agentContext,
                );
              }
            }
            break;

          case "state_change":
            if (action.config.targetState) {
              this.variables = {
                ...this.variables,
                ...action.config.variables,
              };
            }
            break;

          case "workflow_trigger":
            // Integration with WorkflowManager (if available)
            this.logger.debug("Workflow trigger action", action.config);
            break;

          case "custom":
            // Custom action execution
            this.logger.debug("Custom action", action.config);
            break;
        }
      } catch (error) {
        this.logger.error(`Action execution failed: ${action.type}`, {
          error,
          action,
        });
      }
    }
  }
}

/**
 * Agent builder with fluent API
 */
export class AgentBuilder {
  private config: AgentBuilderConfig;
  private agentConfig: Partial<AgentDefinition> = {};
  private conversationalConfig: ConversationalAgentConfig = {};
  private stateMachineConfig?: StateMachineAgentConfig;
  private memoryConfig: MemoryBuilderConfig = { type: "short_term" };
  private tools: ToolBuilderConfig[] = [];
  private logger: Logger;

  constructor(config: AgentBuilderConfig = {}) {
    this.config = config;
    this.logger = config.logger || createComponentLogger("AgentBuilder");

    // Set defaults
    this.agentConfig = {
      name: config.name || "Unnamed Agent",
      type: "conversational",
      config: {
        llmProvider: config.defaultLLMProvider || "gpt-3.5-turbo",
      },
    };
  }

  /**
   * Set agent name
   */
  setName(name: string): this {
    this.agentConfig.name = name;
    return this;
  }

  /**
   * Set agent description
   */
  setDescription(description: string): this {
    this.agentConfig.description = description;
    return this;
  }

  /**
   * Set agent type
   */
  setType(type: AgentType): this {
    this.agentConfig.type = type;
    return this;
  }

  /**
   * Set system prompt for conversational agents
   */
  setSystemPrompt(prompt: string): this {
    this.conversationalConfig.systemPrompt = prompt;
    return this;
  }

  /**
   * Set initial message
   */
  setInitialMessage(message: string): this {
    this.conversationalConfig.initialMessage = message;
    return this;
  }

  /**
   * Set LLM provider
   */
  setLLMProvider(provider: string): this {
    if (!this.agentConfig.config) {
      this.agentConfig.config = {};
    }
    this.agentConfig.config.llmProvider = provider;
    return this;
  }

  /**
   * Add a tool to the agent
   */
  addTool(
    name: string,
    description: string,
    parameters: Record<string, any>,
    implementation: ToolImplementation,
  ): this {
    this.tools.push({
      name,
      description,
      parameters,
      implementation,
    });
    return this;
  }

  /**
   * Add multiple tools
   */
  addTools(
    tools: Omit<ToolBuilderConfig, "implementation">[],
    implementations: Record<string, ToolImplementation>,
  ): this {
    for (const tool of tools) {
      const implementation = implementations[tool.name];
      if (implementation) {
        this.addTool(
          tool.name,
          tool.description,
          tool.parameters,
          implementation,
        );
      }
    }
    return this;
  }

  /**
   * Enable memory
   */
  enableMemory(config: Partial<MemoryBuilderConfig> = {}): this {
    this.memoryConfig = {
      type: "short_term",
      maxSize: 1000,
      persistent: false,
      backend: "memory",
      ...config,
    };
    return this;
  }

  /**
   * Configure conversation settings
   */
  setConversationConfig(
    config: ConversationalAgentConfig["conversation"],
  ): this {
    this.conversationalConfig.conversation = config;
    return this;
  }

  /**
   * Set tool choice strategy
   */
  setToolChoice(choice: ConversationalAgentConfig["toolChoice"]): this {
    this.conversationalConfig.toolChoice = choice;
    return this;
  }

  /**
   * Configure state machine
   */
  setStateMachine(config: StateMachineAgentConfig): this {
    this.agentConfig.type = "state_machine";
    this.stateMachineConfig = config;
    return this;
  }

  /**
   * Add state to state machine
   */
  addState(name: string, state: AgentState): this {
    if (!this.stateMachineConfig) {
      this.stateMachineConfig = {
        initialState: name,
        states: {},
        transitions: [],
      };
    }
    this.stateMachineConfig.states[name] = state;
    return this;
  }

  /**
   * Add transition to state machine
   */
  addTransition(
    fromState: string,
    toState: string,
    trigger?: string,
    condition?: string,
  ): this {
    if (!this.stateMachineConfig) {
      throw new AgentConfigurationError(
        "State machine not initialized. Call setStateMachine() first.",
      );
    }

    this.stateMachineConfig.transitions.push({
      fromState,
      toState,
      trigger,
      condition,
    });
    return this;
  }

  /**
   * Set context window configuration
   */
  setContextWindow(config: ConversationalAgentConfig["contextWindow"]): this {
    this.conversationalConfig.contextWindow = config;
    return this;
  }

  /**
   * Enable debug mode
   */
  enableDebug(): this {
    this.config.debug = true;
    return this;
  }

  /**
   * Validate configuration
   */
  validate(): { isValid: boolean; errors: string[]; warnings: string[] } {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Basic validation
    if (!this.agentConfig.name) {
      errors.push("Agent name is required");
    }

    if (!this.agentConfig.type) {
      errors.push("Agent type is required");
    }

    // Type-specific validation
    if (this.agentConfig.type === "state_machine") {
      if (!this.stateMachineConfig) {
        errors.push(
          "State machine configuration is required for state_machine type",
        );
      } else {
        if (!this.stateMachineConfig.initialState) {
          errors.push("Initial state is required for state machine");
        }

        if (Object.keys(this.stateMachineConfig.states).length === 0) {
          errors.push("At least one state is required for state machine");
        }

        // Check if initial state exists
        if (
          !this.stateMachineConfig.states[this.stateMachineConfig.initialState]
        ) {
          errors.push("Initial state must exist in states configuration");
        }

        // Validate transitions
        for (const transition of this.stateMachineConfig.transitions) {
          if (!this.stateMachineConfig.states[transition.fromState]) {
            errors.push(
              `Transition from state '${transition.fromState}' references non-existent state`,
            );
          }
          if (!this.stateMachineConfig.states[transition.toState]) {
            errors.push(
              `Transition to state '${transition.toState}' references non-existent state`,
            );
          }
        }
      }
    }

    // Tool validation
    const toolNames = new Set();
    for (const tool of this.tools) {
      if (toolNames.has(tool.name)) {
        errors.push(`Duplicate tool name: ${tool.name}`);
      }
      toolNames.add(tool.name);

      if (!tool.name || !tool.description) {
        errors.push("Tool name and description are required");
      }
    }

    // Memory validation
    if (this.memoryConfig.maxSize && this.memoryConfig.maxSize <= 0) {
      errors.push("Memory max size must be positive");
    }

    // Warnings
    if (
      this.agentConfig.type === "conversational" &&
      !this.conversationalConfig.systemPrompt
    ) {
      warnings.push("System prompt recommended for conversational agents");
    }

    if (this.tools.length === 0) {
      warnings.push(
        "No tools configured - agent may have limited capabilities",
      );
    }

    return { isValid: errors.length === 0, errors, warnings };
  }

  /**
   * Build the agent
   */
  async build(llmRouter: LLMRouter): Promise<AgentBuilderResult> {
    // Validate configuration
    const validation = this.validate();
    if (!validation.isValid) {
      throw new AgentConfigurationError(
        `Invalid configuration: ${validation.errors.join(", ")}`,
      );
    }

    // Complete agent configuration
    const agentDefinition: AgentDefinition = {
      id: `agent_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      name: this.agentConfig.name!,
      type: this.agentConfig.type!,
      description: this.agentConfig.description,
      config: {
        ...this.agentConfig.config,
        memory: this.memoryConfig,
      },
      metadata: {
        createdAt: new Date().toISOString(),
        version: "1.0.0",
      },
    };

    // Create agent instance based on type
    let agent: Agent;

    if (this.agentConfig.type === "conversational") {
      agent = new ConversationalAgent(
        agentDefinition,
        this.conversationalConfig,
        llmRouter,
        this.logger,
      );
    } else if (this.agentConfig.type === "state_machine") {
      if (!this.stateMachineConfig) {
        throw new AgentConfigurationError(
          "State machine configuration required",
        );
      }
      agent = new StateMachineAgent(
        agentDefinition,
        this.stateMachineConfig,
        llmRouter,
        this.logger,
      );
    } else {
      throw new AgentConfigurationError(
        `Unsupported agent type: ${this.agentConfig.type}`,
      );
    }

    // Add tools to agent
    for (const tool of this.tools) {
      agent.addTool(tool);
    }

    this.logger.info(`Agent built successfully: ${agent.name} (${agent.type})`);

    return {
      agent,
      config: agentDefinition,
      validation,
    };
  }

  /**
   * Clone the builder with current configuration
   */
  clone(): AgentBuilder {
    const cloned = new AgentBuilder(this.config);
    cloned.agentConfig = { ...this.agentConfig };
    cloned.conversationalConfig = { ...this.conversationalConfig };
    cloned.stateMachineConfig = this.stateMachineConfig
      ? { ...this.stateMachineConfig }
      : undefined;
    cloned.memoryConfig = { ...this.memoryConfig };
    cloned.tools = [...this.tools];
    return cloned;
  }

  /**
   * Export configuration as JSON
   */
  toJSON(): any {
    return {
      config: this.config,
      agentConfig: this.agentConfig,
      conversationalConfig: this.conversationalConfig,
      stateMachineConfig: this.stateMachineConfig,
      memoryConfig: this.memoryConfig,
      tools: this.tools.map((tool) => ({
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
        config: tool.config,
      })),
    };
  }

  /**
   * Import configuration from JSON
   */
  static fromJSON(json: any): AgentBuilder {
    const builder = new AgentBuilder(json.config);

    if (json.agentConfig) {
      builder.agentConfig = json.agentConfig;
    }

    if (json.conversationalConfig) {
      builder.conversationalConfig = json.conversationalConfig;
    }

    if (json.stateMachineConfig) {
      builder.stateMachineConfig = json.stateMachineConfig;
    }

    if (json.memoryConfig) {
      builder.memoryConfig = json.memoryConfig;
    }

    return builder;
  }
}

/**
 * Specialized builders for common agent types
 */
export class ConversationalAgentBuilder extends AgentBuilder {
  constructor(systemPrompt: string, config: AgentBuilderConfig = {}) {
    super(config);
    this.setType("conversational")
      .setSystemPrompt(systemPrompt)
      .enableMemory({ type: "both", maxSize: 1000 });
  }

  /**
   * Add customer support capabilities
   */
  addCustomerSupportTools(): this {
    return this.addTool(
      "search_knowledge_base",
      "Search the knowledge base for relevant information",
      {
        type: "object",
        properties: {
          query: { type: "string", description: "Search query" },
          category: { type: "string", description: "Optional category filter" },
        },
        required: ["query"],
      },
      async (params, context) => {
        // Mock implementation
        return {
          results: [
            {
              title: "FAQ Answer",
              content: "Sample knowledge base result",
              relevance: 0.9,
            },
          ],
        };
      },
    ).addTool(
      "create_support_ticket",
      "Create a support ticket for customer issues",
      {
        type: "object",
        properties: {
          title: { type: "string", description: "Ticket title" },
          description: { type: "string", description: "Issue description" },
          priority: {
            type: "string",
            enum: ["low", "medium", "high"],
            description: "Ticket priority",
          },
        },
        required: ["title", "description"],
      },
      async (params, context) => {
        // Mock implementation
        return {
          ticketId: `TICKET-${Date.now()}`,
          status: "created",
          message: "Support ticket created successfully",
        };
      },
    );
  }

  /**
   * Add sales assistant capabilities
   */
  addSalesTools(): this {
    return this.addTool(
      "get_product_info",
      "Get detailed information about products",
      {
        type: "object",
        properties: {
          productId: { type: "string", description: "Product ID" },
          includePrice: {
            type: "boolean",
            description: "Include pricing information",
          },
        },
        required: ["productId"],
      },
      async (params, context) => {
        // Mock implementation
        return {
          product: {
            id: params.productId,
            name: "Sample Product",
            price: "$99.99",
            description: "A great product for your needs",
            inStock: true,
          },
        };
      },
    ).addTool(
      "calculate_quote",
      "Calculate pricing quote for customer",
      {
        type: "object",
        properties: {
          items: {
            type: "array",
            items: {
              type: "object",
              properties: {
                productId: { type: "string" },
                quantity: { type: "number" },
              },
              required: ["productId", "quantity"],
            },
          },
          discountCode: {
            type: "string",
            description: "Optional discount code",
          },
        },
        required: ["items"],
      },
      async (params, context) => {
        // Mock implementation
        const total = params.items.reduce(
          (sum: number, item: any) => sum + item.quantity * 99.99,
          0,
        );
        return {
          subtotal: total,
          discount: params.discountCode ? total * 0.1 : 0,
          total: params.discountCode ? total * 0.9 : total,
          quoteId: `QUOTE-${Date.now()}`,
        };
      },
    );
  }
}

export class WorkflowAgentBuilder extends AgentBuilder {
  constructor(config: AgentBuilderConfig = {}) {
    super(config);
    this.setType("state_machine").enableMemory({
      type: "both",
      persistent: true,
    });
  }

  /**
   * Create an order processing workflow agent
   */
  createOrderProcessingAgent(): this {
    return this.setName("Order Processing Agent")
      .addState("initial", {
        name: "initial",
        prompt:
          "Analyze the incoming order request and extract relevant information.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["validate_order", "request_info"],
      })
      .addState("validate_order", {
        name: "validate_order",
        prompt:
          "Validate the order information including inventory, pricing, and customer details.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["process_payment", "request_correction"],
      })
      .addState("process_payment", {
        name: "process_payment",
        prompt: "Process the payment for the validated order.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["fulfill_order", "payment_failed"],
      })
      .addState("fulfill_order", {
        name: "fulfill_order",
        prompt: "Coordinate order fulfillment and shipping.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["completed"],
      })
      .addState("completed", {
        name: "completed",
        prompt: "Order processing completed successfully.",
        defaultResponse: "Order has been processed and is being fulfilled.",
        onEntry: [],
        onExit: [],
        availableTransitions: [],
      })
      .addTransition("initial", "validate_order", "order_info_complete")
      .addTransition("validate_order", "process_payment", "validation_passed")
      .addTransition("process_payment", "fulfill_order", "payment_successful")
      .addTransition("fulfill_order", "completed", "fulfillment_initiated");
  }

  /**
   * Create an approval workflow agent
   */
  createApprovalWorkflowAgent(): this {
    return this.setName("Approval Workflow Agent")
      .addState("submitted", {
        name: "submitted",
        prompt:
          "Review the submitted request and determine if it requires approval.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["auto_approve", "manager_review", "rejected"],
      })
      .addState("manager_review", {
        name: "manager_review",
        prompt:
          "Request is under manager review. Waiting for approval decision.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["approved", "rejected", "escalated"],
      })
      .addState("escalated", {
        name: "escalated",
        prompt: "Request has been escalated to senior management.",
        onEntry: [],
        onExit: [],
        availableTransitions: ["approved", "rejected"],
      })
      .addState("approved", {
        name: "approved",
        prompt: "Request has been approved.",
        defaultResponse:
          "Your request has been approved and is being processed.",
        onEntry: [],
        onExit: [],
        availableTransitions: [],
      })
      .addState("rejected", {
        name: "rejected",
        prompt: "Request has been rejected.",
        defaultResponse:
          "Your request has been rejected. Please review and resubmit if necessary.",
        onEntry: [],
        onExit: [],
        availableTransitions: [],
      })
      .addTransition("submitted", "auto_approve", "amount < 1000")
      .addTransition(
        "submitted",
        "manager_review",
        "amount >= 1000 && amount < 10000",
      )
      .addTransition("submitted", "escalated", "amount >= 10000")
      .addTransition("manager_review", "approved", "manager_approved")
      .addTransition("manager_review", "rejected", "manager_rejected")
      .addTransition("manager_review", "escalated", "requires_escalation");
  }
}

/**
 * Factory functions for creating agents
 */
export function createAgent(config: AgentBuilderConfig = {}): AgentBuilder {
  return new AgentBuilder(config);
}

export function createConversationalAgent(
  systemPrompt: string,
  config: AgentBuilderConfig = {},
): ConversationalAgentBuilder {
  return new ConversationalAgentBuilder(systemPrompt, config);
}

export function createWorkflowAgent(
  config: AgentBuilderConfig = {},
): WorkflowAgentBuilder {
  return new WorkflowAgentBuilder(config);
}

/**
 * Agent templates for common use cases
 */
export const AgentTemplates = {
  /**
   * Customer support agent template
   */
  customerSupport: (
    config: AgentBuilderConfig = {},
  ): ConversationalAgentBuilder => {
    return createConversationalAgent(
      `You are a helpful customer support agent. Your goal is to assist customers with their inquiries, resolve issues, and provide excellent service. You have access to tools for searching knowledge bases and creating support tickets when needed.

Guidelines:
- Be polite, professional, and empathetic
- Listen carefully to customer concerns
- Try to resolve issues yourself first
- Escalate complex issues by creating support tickets
- Always follow up to ensure customer satisfaction`,
      config,
    ).addCustomerSupportTools();
  },

  /**
   * Sales assistant agent template
   */
  salesAssistant: (
    config: AgentBuilderConfig = {},
  ): ConversationalAgentBuilder => {
    return createConversationalAgent(
      `You are a knowledgeable sales assistant. Your role is to help customers find the right products, answer questions about features and pricing, and guide them through the purchase process.

Guidelines:
- Understand customer needs and preferences
- Provide accurate product information
- Offer relevant recommendations
- Calculate quotes when requested
- Be consultative, not pushy
- Focus on customer value and satisfaction`,
      config,
    ).addSalesTools();
  },

  /**
   * Technical support agent template
   */
  technicalSupport: (
    config: AgentBuilderConfig = {},
  ): ConversationalAgentBuilder => {
    return createConversationalAgent(
      `You are a technical support specialist. You help customers troubleshoot technical issues, provide step-by-step guidance, and ensure their technical problems are resolved.

Guidelines:
- Ask clarifying questions to understand the issue
- Provide clear, step-by-step instructions
- Use simple language, avoid technical jargon
- Be patient and thorough
- Verify solutions work before closing
- Document solutions for future reference`,
      config,
    ).addTool(
      "run_diagnostic",
      "Run diagnostic tests on customer systems",
      {
        type: "object",
        properties: {
          testType: { type: "string", description: "Type of diagnostic test" },
          targetSystem: { type: "string", description: "System to test" },
        },
        required: ["testType"],
      },
      async (params, context) => {
        return {
          testId: `DIAG-${Date.now()}`,
          results: { status: "passed", details: "All systems operational" },
          recommendations: ["Update drivers", "Clear cache"],
        };
      },
    );
  },

  /**
   * Data analysis agent template
   */
  dataAnalyst: (
    config: AgentBuilderConfig = {},
  ): ConversationalAgentBuilder => {
    return createConversationalAgent(
      `You are a data analysis specialist. You help users understand their data, create reports, identify trends, and provide insights for decision-making.

Guidelines:
- Ask about data sources and analysis goals
- Suggest appropriate analysis methods
- Explain findings in business terms
- Provide actionable recommendations
- Visualize data when helpful
- Ensure data privacy and security`,
      config,
    ).addTool(
      "analyze_data",
      "Perform data analysis on provided datasets",
      {
        type: "object",
        properties: {
          dataSource: { type: "string", description: "Data source identifier" },
          analysisType: {
            type: "string",
            description: "Type of analysis to perform",
          },
          parameters: { type: "object", description: "Analysis parameters" },
        },
        required: ["dataSource", "analysisType"],
      },
      async (params, context) => {
        return {
          analysisId: `ANALYSIS-${Date.now()}`,
          results: {
            summary: "Data analysis completed",
            insights: ["Trend A increasing", "Pattern B detected"],
          },
          visualizations: ["chart1.png", "graph2.png"],
        };
      },
    );
  },

  /**
   * Order processing workflow template
   */
  orderProcessing: (config: AgentBuilderConfig = {}): WorkflowAgentBuilder => {
    return createWorkflowAgent(config).createOrderProcessingAgent();
  },

  /**
   * Approval workflow template
   */
  approvalWorkflow: (config: AgentBuilderConfig = {}): WorkflowAgentBuilder => {
    return createWorkflowAgent(config).createApprovalWorkflowAgent();
  },
};
