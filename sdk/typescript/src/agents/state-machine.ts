/**
 * State Machine Agent Implementation for Circuit Breaker SDK
 *
 * Provides workflow-driven conversational AI with:
 * - State-based conversation flow
 * - Conditional transitions and branching
 * - Action execution on state changes
 * - Variable management and persistence
 * - Integration with external workflows
 * - Complex business logic support
 *
 * @example
 * ```typescript
 * const agent = new StateMachineAgent({
 *   name: 'Order Processing Agent',
 *   initialState: 'greeting',
 *   states: {
 *     greeting: {
 *       name: 'greeting',
 *       prompt: 'Welcome! How can I help you with your order?',
 *       onEntry: [{ type: 'function_call', config: { functionName: 'logInteraction' } }]
 *     },
 *     collecting_info: {
 *       name: 'collecting_info',
 *       prompt: 'Please provide your order details.',
 *       availableTransitions: ['validate_order', 'request_clarification']
 *     }
 *   },
 *   transitions: [
 *     { fromState: 'greeting', toState: 'collecting_info', trigger: 'order' }
 *   ]
 * }, llmRouter);
 * ```
 */

import {
  AgentDefinition,
  AgentConfig,
  StateMachineConfig,
  AgentState,
  StateTransition,
  StateAction,
  ChatCompletionRequest,
  ChatMessage,
  ChatRole,
} from "../core/types.js";
import {
  AgentError,
  AgentConfigurationError,
  LLMError,
  ValidationError,
} from "../core/errors.js";
import { Logger, createComponentLogger } from "../utils/logger.js";
import { LLMRouter } from "../llm/router.js";
import {
  Agent,
  AgentContext,
  MemoryManager,
  ToolBuilderConfig,
} from "./builder.js";

export interface StateMachineAgentConfig {
  /** Initial state name */
  initialState: string;

  /** Available states */
  states: Record<string, AgentState>;

  /** State transitions */
  transitions: StateTransition[];

  /** Global variables */
  variables?: Record<string, any>;

  /** State timeout settings (in seconds) */
  timeouts?: Record<string, number>;

  /** Transition evaluation settings */
  evaluation?: {
    enableJavaScript?: boolean;
    enableRegex?: boolean;
    enableCustomFunctions?: boolean;
    timeout?: number; // ms
  };

  /** Error handling */
  errorHandling?: {
    fallbackState?: string;
    maxRetries?: number;
    retryDelay?: number; // ms
  };

  /** Workflow integration */
  workflow?: {
    enableWorkflowTriggers?: boolean;
    workflowEndpoint?: string;
    enableStateSync?: boolean;
  };
}

export interface StateExecutionContext {
  currentState: string;
  previousState?: string;
  variables: Record<string, any>;
  userMessage: string;
  agentResponse?: string;
  sessionData: Record<string, any>;
  metadata: {
    stateEntryTime: Date;
    turnCount: number;
    totalTimeInState: number;
  };
}

export interface StateTransitionResult {
  success: boolean;
  newState: string;
  triggeredBy: "condition" | "trigger" | "timeout" | "manual";
  executedActions: string[];
  errors?: string[];
}

export interface StateMachineMetrics {
  totalTransitions: number;
  stateVisitCounts: Record<string, number>;
  averageTimePerState: Record<string, number>;
  transitionSuccessRate: number;
  errorCount: number;
  completedFlows: number;
  abandonedFlows: number;
}

export interface StateMachineSession {
  id: string;
  userId?: string;
  currentState: string;
  stateHistory: Array<{
    state: string;
    entryTime: Date;
    exitTime?: Date;
    triggeredBy?: string;
  }>;
  variables: Record<string, any>;
  lastActivity: Date;
  status: "active" | "completed" | "error" | "timeout";
  flowStartTime: Date;
  metadata: Record<string, any>;
}

/**
 * State machine agent for workflow-driven conversations
 */
export class StateMachineAgent extends Agent {
  private config: StateMachineAgentConfig;
  private sessions: Map<string, StateMachineSession> = new Map();
  private metrics: StateMachineMetrics;
  private stateTimeouts: Map<string, NodeJS.Timeout> = new Map();
  private customEvaluators: Map<string, Function> = new Map();

  constructor(
    agentDefinition: AgentDefinition,
    config: StateMachineAgentConfig,
    llmRouter: LLMRouter,
    logger?: Logger,
  ) {
    super(agentDefinition, llmRouter, logger);

    this.config = {
      evaluation: {
        enableJavaScript: false,
        enableRegex: true,
        enableCustomFunctions: true,
        timeout: 5000,
        ...config.evaluation,
      },
      errorHandling: {
        maxRetries: 3,
        retryDelay: 1000,
        ...config.errorHandling,
      },
      workflow: {
        enableWorkflowTriggers: false,
        enableStateSync: false,
        ...config.workflow,
      },
      ...config,
    };

    this.validateConfiguration();
    this.initializeMetrics();
  }

  /**
   * Process a chat message through the state machine
   */
  async chat(
    message: string,
    context?: Partial<AgentContext>,
  ): Promise<string> {
    const sessionId = context?.conversationId || "default";
    const userId = context?.userId || "anonymous";

    try {
      // Get or create session
      let session = this.sessions.get(sessionId);
      if (!session) {
        session = this.createSession(sessionId, userId);
      }

      // Update session activity
      session.lastActivity = new Date();

      // Create execution context
      const executionContext: StateExecutionContext = {
        currentState: session.currentState,
        previousState:
          session.stateHistory[session.stateHistory.length - 1]?.state,
        variables: { ...session.variables },
        userMessage: message,
        sessionData: context?.session || {},
        metadata: {
          stateEntryTime:
            session.stateHistory[session.stateHistory.length - 1]?.entryTime ||
            new Date(),
          turnCount: session.stateHistory.length,
          totalTimeInState:
            Date.now() -
            (session.stateHistory[
              session.stateHistory.length - 1
            ]?.entryTime.getTime() || Date.now()),
        },
      };

      // Get current state configuration
      const currentStateConfig = this.config.states[session.currentState];
      if (!currentStateConfig) {
        throw new AgentError(`Invalid state: ${session.currentState}`);
      }

      // Execute state entry actions if this is a new state entry
      const lastHistoryEntry =
        session.stateHistory[session.stateHistory.length - 1];
      if (
        !lastHistoryEntry ||
        lastHistoryEntry.state !== session.currentState
      ) {
        await this.executeStateActions(
          currentStateConfig.onEntry || [],
          executionContext,
          context,
        );
      }

      // Generate response using LLM if state has prompt
      let response = "";
      if (currentStateConfig.prompt) {
        response = await this.generateStateResponse(
          currentStateConfig,
          executionContext,
          context,
        );
        executionContext.agentResponse = response;
      }

      // Evaluate potential state transitions
      const transitionResult = await this.evaluateTransitions(
        executionContext,
        context,
      );

      if (
        transitionResult.success &&
        transitionResult.newState !== session.currentState
      ) {
        // Execute transition
        await this.executeStateTransition(
          session,
          transitionResult,
          executionContext,
          context,
        );
      }

      // Update session variables
      session.variables = executionContext.variables;

      // Set state timeout if configured
      this.setStateTimeout(session, currentStateConfig);

      // Update metrics
      this.updateMetrics(session, transitionResult);

      return (
        response ||
        currentStateConfig.defaultResponse ||
        "State processed successfully."
      );
    } catch (error) {
      await this.handleError(sessionId, error as Error, context);
      throw new AgentError(
        `State machine processing failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  /**
   * Manually transition to a specific state
   */
  async transitionToState(
    sessionId: string,
    targetState: string,
    context?: Partial<AgentContext>,
  ): Promise<StateTransitionResult> {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new AgentError(`Session not found: ${sessionId}`);
    }

    if (!this.config.states[targetState]) {
      throw new AgentError(`Target state not found: ${targetState}`);
    }

    const executionContext: StateExecutionContext = {
      currentState: session.currentState,
      variables: { ...session.variables },
      userMessage: "",
      sessionData: context?.session || {},
      metadata: {
        stateEntryTime: new Date(),
        turnCount: session.stateHistory.length,
        totalTimeInState: 0,
      },
    };

    const transitionResult: StateTransitionResult = {
      success: true,
      newState: targetState,
      triggeredBy: "manual",
      executedActions: [],
    };

    await this.executeStateTransition(
      session,
      transitionResult,
      executionContext,
      context,
    );
    return transitionResult;
  }

  /**
   * Get current state for a session
   */
  getSessionState(sessionId: string): StateMachineSession | undefined {
    return this.sessions.get(sessionId);
  }

  /**
   * Get all active sessions
   */
  getActiveSessions(): StateMachineSession[] {
    return Array.from(this.sessions.values()).filter(
      (s) => s.status === "active",
    );
  }

  /**
   * Get state machine metrics
   */
  getMetrics(): StateMachineMetrics {
    return { ...this.metrics };
  }

  /**
   * Reset a session to initial state
   */
  async resetSession(sessionId: string): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (session) {
      // Clear timeouts
      this.clearStateTimeout(sessionId);

      // Reset to initial state
      session.currentState = this.config.initialState;
      session.stateHistory = [
        {
          state: this.config.initialState,
          entryTime: new Date(),
          triggeredBy: "reset",
        },
      ];
      session.variables = { ...this.config.variables };
      session.status = "active";
      session.lastActivity = new Date();

      this.logger.info("Session reset", {
        sessionId,
        initialState: this.config.initialState,
      });
    }
  }

  /**
   * Add custom transition evaluator
   */
  addCustomEvaluator(name: string, evaluator: Function): void {
    this.customEvaluators.set(name, evaluator);
    this.logger.debug("Custom evaluator added", { name });
  }

  private createSession(
    sessionId: string,
    userId?: string,
  ): StateMachineSession {
    const session: StateMachineSession = {
      id: sessionId,
      userId,
      currentState: this.config.initialState,
      stateHistory: [
        {
          state: this.config.initialState,
          entryTime: new Date(),
          triggeredBy: "initial",
        },
      ],
      variables: { ...this.config.variables },
      lastActivity: new Date(),
      status: "active",
      flowStartTime: new Date(),
      metadata: {},
    };

    this.sessions.set(sessionId, session);
    this.logger.info("New state machine session created", {
      sessionId,
      userId,
      initialState: this.config.initialState,
    });
    return session;
  }

  private async generateStateResponse(
    stateConfig: AgentState,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<string> {
    const systemPrompt = this.buildStatePrompt(stateConfig, context);

    const request: ChatCompletionRequest = {
      model: this.config.config?.llmProvider || "gpt-3.5-turbo",
      messages: [
        { role: "system", content: systemPrompt },
        { role: "user", content: context.userMessage },
      ],
      temperature: 0.3,
      max_tokens: 500,
      tools: this.getToolsForLLM(),
      tool_choice: this.tools.size > 0 ? "auto" : undefined,
    };

    const response = await this.llmRouter.chatCompletion(request);
    return response.choices[0].message.content || "";
  }

  private buildStatePrompt(
    stateConfig: AgentState,
    context: StateExecutionContext,
  ): string {
    let prompt = stateConfig.prompt || "";

    // Replace variables in prompt
    for (const [key, value] of Object.entries(context.variables)) {
      prompt = prompt.replace(new RegExp(`{{${key}}}`, "g"), String(value));
    }

    // Add context information
    prompt += `\n\nCurrent state: ${context.currentState}`;
    if (context.previousState) {
      prompt += `\nPrevious state: ${context.previousState}`;
    }
    prompt += `\nUser message: ${context.userMessage}`;

    // Add available transitions
    if (stateConfig.availableTransitions?.length) {
      prompt += `\nAvailable transitions: ${stateConfig.availableTransitions.join(", ")}`;
    }

    // Add state-specific variables
    const stateVars = Object.entries(context.variables)
      .filter(([key]) => key.startsWith(`${context.currentState}_`))
      .map(([key, value]) => `${key}: ${value}`)
      .join(", ");
    if (stateVars) {
      prompt += `\nState variables: ${stateVars}`;
    }

    return prompt;
  }

  private async evaluateTransitions(
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<StateTransitionResult> {
    const availableTransitions = this.config.transitions.filter(
      (t) => t.fromState === context.currentState,
    );

    for (const transition of availableTransitions) {
      try {
        let shouldTransition = false;
        let triggeredBy: "condition" | "trigger" | "timeout" | "manual" =
          "condition";

        // Check trigger patterns
        if (
          transition.trigger &&
          this.matchesTrigger(transition.trigger, context.userMessage)
        ) {
          shouldTransition = true;
          triggeredBy = "trigger";
        }

        // Evaluate conditions
        if (!shouldTransition && transition.condition) {
          shouldTransition = await this.evaluateCondition(
            transition.condition,
            context,
            agentContext,
          );
          triggeredBy = "condition";
        }

        if (shouldTransition) {
          return {
            success: true,
            newState: transition.toState,
            triggeredBy,
            executedActions: [],
          };
        }
      } catch (error) {
        this.logger.warn("Transition evaluation failed", {
          transition: transition.fromState + " -> " + transition.toState,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    return {
      success: false,
      newState: context.currentState,
      triggeredBy: "condition",
      executedActions: [],
    };
  }

  private async evaluateCondition(
    condition: string,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<boolean> {
    try {
      // Create evaluation context
      const evalContext = {
        userMessage: context.userMessage,
        agentResponse: context.agentResponse || "",
        variables: context.variables,
        currentState: context.currentState,
        previousState: context.previousState,
        turnCount: context.metadata.turnCount,
        timeInState: context.metadata.totalTimeInState,
        sessionData: context.sessionData,
      };

      // Handle custom function calls
      if (
        this.config.evaluation?.enableCustomFunctions &&
        condition.includes("custom.")
      ) {
        return await this.evaluateCustomFunction(
          condition,
          evalContext,
          agentContext,
        );
      }

      // Handle regex patterns
      if (
        this.config.evaluation?.enableRegex &&
        condition.startsWith("regex:")
      ) {
        const pattern = condition.substring(6);
        const regex = new RegExp(pattern, "i");
        return regex.test(context.userMessage);
      }

      // Handle JavaScript evaluation (if enabled and safe)
      if (this.config.evaluation?.enableJavaScript) {
        return this.evaluateJavaScriptCondition(condition, evalContext);
      }

      // Simple string comparison fallback
      return this.evaluateSimpleCondition(condition, evalContext);
    } catch (error) {
      this.logger.warn("Condition evaluation failed", { condition, error });
      return false;
    }
  }

  private async evaluateCustomFunction(
    condition: string,
    context: any,
    agentContext?: Partial<AgentContext>,
  ): Promise<boolean> {
    const match = condition.match(/custom\.(\w+)\((.*)\)/);
    if (!match) return false;

    const [, functionName, argsStr] = match;
    const evaluator = this.customEvaluators.get(functionName);
    if (!evaluator) return false;

    try {
      const args = argsStr ? JSON.parse(`[${argsStr}]`) : [];
      return await evaluator(context, ...args);
    } catch (error) {
      this.logger.warn("Custom function evaluation failed", {
        functionName,
        error,
      });
      return false;
    }
  }

  private evaluateJavaScriptCondition(
    condition: string,
    context: any,
  ): boolean {
    // WARNING: This is potentially unsafe. In production, use a secure sandboxed evaluator
    try {
      // Replace context variables
      let evalCode = condition;
      for (const [key, value] of Object.entries(context)) {
        const regex = new RegExp(`\\b${key}\\b`, "g");
        evalCode = evalCode.replace(regex, JSON.stringify(value));
      }

      // Use Function constructor instead of eval for slightly better security
      const result = new Function(`return (${evalCode})`)();
      return Boolean(result);
    } catch (error) {
      this.logger.warn("JavaScript condition evaluation failed", {
        condition,
        error,
      });
      return false;
    }
  }

  private evaluateSimpleCondition(condition: string, context: any): boolean {
    // Handle simple comparisons like "amount > 1000"
    const operators = [
      ">=",
      "<=",
      "==",
      "!=",
      ">",
      "<",
      "includes",
      "startsWith",
      "endsWith",
    ];

    for (const op of operators) {
      if (condition.includes(op)) {
        const [left, right] = condition.split(op).map((s) => s.trim());
        const leftValue = context[left] || left;
        const rightValue = context[right] || right;

        switch (op) {
          case ">":
            return Number(leftValue) > Number(rightValue);
          case "<":
            return Number(leftValue) < Number(rightValue);
          case ">=":
            return Number(leftValue) >= Number(rightValue);
          case "<=":
            return Number(leftValue) <= Number(rightValue);
          case "==":
            return leftValue == rightValue;
          case "!=":
            return leftValue != rightValue;
          case "includes":
            return String(leftValue).includes(String(rightValue));
          case "startsWith":
            return String(leftValue).startsWith(String(rightValue));
          case "endsWith":
            return String(leftValue).endsWith(String(rightValue));
        }
      }
    }

    return false;
  }

  private matchesTrigger(trigger: string, userMessage: string): boolean {
    const lowerMessage = userMessage.toLowerCase();
    const lowerTrigger = trigger.toLowerCase();

    // Support both exact word matching and substring matching
    if (trigger.startsWith("/") && trigger.endsWith("/")) {
      // Regex pattern
      const pattern = trigger.slice(1, -1);
      const regex = new RegExp(pattern, "i");
      return regex.test(userMessage);
    }

    // Simple substring or word matching
    return (
      lowerMessage.includes(lowerTrigger) ||
      lowerMessage.split(/\s+/).includes(lowerTrigger)
    );
  }

  private async executeStateTransition(
    session: StateMachineSession,
    transitionResult: StateTransitionResult,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    const currentState = this.config.states[session.currentState];
    const targetState = this.config.states[transitionResult.newState];

    this.logger.debug("Executing state transition", {
      sessionId: session.id,
      from: session.currentState,
      to: transitionResult.newState,
      triggeredBy: transitionResult.triggeredBy,
    });

    // Execute exit actions for current state
    if (currentState?.onExit) {
      await this.executeStateActions(
        currentState.onExit,
        context,
        agentContext,
      );
    }

    // Update session state
    const previousState = session.currentState;
    session.currentState = transitionResult.newState;

    // Update state history
    const lastHistoryEntry =
      session.stateHistory[session.stateHistory.length - 1];
    if (lastHistoryEntry && !lastHistoryEntry.exitTime) {
      lastHistoryEntry.exitTime = new Date();
    }

    session.stateHistory.push({
      state: transitionResult.newState,
      entryTime: new Date(),
      triggeredBy: transitionResult.triggeredBy,
    });

    // Clear previous state timeout
    this.clearStateTimeout(session.id);

    // Execute entry actions for new state
    if (targetState?.onEntry) {
      const newContext = {
        ...context,
        currentState: transitionResult.newState,
        previousState,
      };
      await this.executeStateActions(
        targetState.onEntry,
        newContext,
        agentContext,
      );
    }

    // Check for workflow integration
    if (this.config.workflow?.enableWorkflowTriggers) {
      await this.triggerWorkflowEvent(session, transitionResult, context);
    }

    this.metrics.totalTransitions++;
    this.metrics.stateVisitCounts[transitionResult.newState] =
      (this.metrics.stateVisitCounts[transitionResult.newState] || 0) + 1;
  }

  private async executeStateActions(
    actions: StateAction[],
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    for (const action of actions) {
      try {
        await this.executeAction(action, context, agentContext);
      } catch (error) {
        this.logger.error("State action execution failed", {
          action: action.type,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }
  }

  private async executeAction(
    action: StateAction,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    switch (action.type) {
      case "function_call":
        await this.executeFunctionAction(action, context, agentContext);
        break;

      case "state_change":
        this.executeStateChangeAction(action, context);
        break;

      case "workflow_trigger":
        await this.executeWorkflowTriggerAction(action, context, agentContext);
        break;

      case "custom":
        await this.executeCustomAction(action, context, agentContext);
        break;

      default:
        this.logger.warn("Unknown action type", { type: action.type });
    }
  }

  private async executeFunctionAction(
    action: StateAction,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    const functionName = action.config.functionName;
    if (!functionName) return;

    const tool = this.tools.get(functionName);
    if (tool) {
      const fullContext: AgentContext = {
        agent: this,
        conversationId: agentContext?.conversationId || "default",
        userId: agentContext?.userId || "anonymous",
        session: { ...context.sessionData, ...agentContext?.session },
        memory: this.memory,
        llm: this.llmRouter,
        logger: this.logger,
      };

      const result = await tool.implementation(
        action.config.parameters || {},
        fullContext,
      );

      // Store result in variables if specified
      if (action.config.resultVariable) {
        context.variables[action.config.resultVariable] = result;
      }
    }
  }

  private executeStateChangeAction(
    action: StateAction,
    context: StateExecutionContext,
  ): void {
    if (action.config.variables) {
      Object.assign(context.variables, action.config.variables);
    }

    if (action.config.setValue) {
      const { variable, value } = action.config.setValue;
      context.variables[variable] = value;
    }
  }

  private async executeWorkflowTriggerAction(
    action: StateAction,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    this.logger.debug("Workflow trigger action executed", {
      workflowName: action.config.workflowName,
      parameters: action.config.parameters,
      currentState: context.currentState,
    });

    // This would integrate with the WorkflowManager if available
    // For now, just log the trigger
  }

  private async executeCustomAction(
    action: StateAction,
    context: StateExecutionContext,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    this.logger.debug("Custom action executed", {
      actionName: action.config.actionName,
      parameters: action.config.parameters,
      currentState: context.currentState,
    });

    // Custom action execution would be implemented here
  }

  private setStateTimeout(
    session: StateMachineSession,
    stateConfig: AgentState,
  ): void {
    const timeout =
      this.config.timeouts?.[session.currentState] || stateConfig.timeout;
    if (timeout) {
      const timeoutHandle = setTimeout(() => {
        this.handleStateTimeout(session.id);
      }, timeout * 1000);

      this.stateTimeouts.set(session.id, timeoutHandle);
    }
  }

  private clearStateTimeout(sessionId: string): void {
    const timeoutHandle = this.stateTimeouts.get(sessionId);
    if (timeoutHandle) {
      clearTimeout(timeoutHandle);
      this.stateTimeouts.delete(sessionId);
    }
  }

  private async handleStateTimeout(sessionId: string): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) return;

    this.logger.warn("State timeout occurred", {
      sessionId,
      currentState: session.currentState,
    });

    // Find timeout transition or use fallback
    const timeoutTransition = this.config.transitions.find(
      (t) => t.fromState === session.currentState && t.trigger === "timeout",
    );

    if (timeoutTransition) {
      const context: StateExecutionContext = {
        currentState: session.currentState,
        variables: session.variables,
        userMessage: "",
        sessionData: {},
        metadata: {
          stateEntryTime: new Date(),
          turnCount: session.stateHistory.length,
          totalTimeInState: 0,
        },
      };

      const transitionResult: StateTransitionResult = {
        success: true,
        newState: timeoutTransition.toState,
        triggeredBy: "timeout",
        executedActions: [],
      };

      await this.executeStateTransition(session, transitionResult, context);
    } else if (this.config.errorHandling?.fallbackState) {
      session.currentState = this.config.errorHandling.fallbackState;
      session.status = "error";
    }
  }

  private async triggerWorkflowEvent(
    session: StateMachineSession,
    transitionResult: StateTransitionResult,
    context: StateExecutionContext,
  ): Promise<void> {
    // This would integrate with external workflow systems
    this.logger.debug("Workflow event triggered", {
      sessionId: session.id,
      event: "state_transition",
      from: context.currentState,
      to: transitionResult.newState,
    });
  }

  private async handleError(
    sessionId: string,
    error: Error,
    agentContext?: Partial<AgentContext>,
  ): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) return;

    this.metrics.errorCount++;

    this.logger.error("State machine error", {
      sessionId,
      currentState: session.currentState,
      error: error.message,
    });

    // Transition to fallback state if configured
    if (this.config.errorHandling?.fallbackState) {
      const context: StateExecutionContext = {
        currentState: session.currentState,
        variables: session.variables,
        userMessage: "",
        sessionData: agentContext?.session || {},
        metadata: {
          stateEntryTime: new Date(),
          turnCount: session.stateHistory.length,
          totalTimeInState: 0,
        },
      };

      const transitionResult: StateTransitionResult = {
        success: true,
        newState: this.config.errorHandling.fallbackState,
        triggeredBy: "manual",
        executedActions: [],
        errors: [error.message],
      };

      await this.executeStateTransition(
        session,
        transitionResult,
        context,
        agentContext,
      );
      session.status = "error";
    }
  }

  private validateConfiguration(): void {
    // Validate initial state exists
    if (!this.config.states[this.config.initialState]) {
      throw new AgentConfigurationError(
        `Initial state '${this.config.initialState}' not found in states`,
      );
    }

    // Validate transitions reference existing states
    for (const transition of this.config.transitions) {
      if (!this.config.states[transition.fromState]) {
        throw new AgentConfigurationError(
          `Transition from state '${transition.fromState}' references non-existent state`,
        );
      }
      if (!this.config.states[transition.toState]) {
        throw new AgentConfigurationError(
          `Transition to state '${transition.toState}' references non-existent state`,
        );
      }
    }

    // Validate fallback state if specified
    if (
      this.config.errorHandling?.fallbackState &&
      !this.config.states[this.config.errorHandling.fallbackState]
    ) {
      throw new AgentConfigurationError(
        `Fallback state '${this.config.errorHandling.fallbackState}' not found in states`,
      );
    }
  }

  private initializeMetrics(): void {
    this.metrics = {
      totalTransitions: 0,
      stateVisitCounts: {},
      averageTimePerState: {},
      transitionSuccessRate: 0,
      errorCount: 0,
      completedFlows: 0,
      abandonedFlows: 0,
    };

    // Initialize visit counts for all states
    for (const stateName of Object.keys(this.config.states)) {
      this.metrics.stateVisitCounts[stateName] = 0;
      this.metrics.averageTimePerState[stateName] = 0;
    }
  }

  private updateMetrics(
    session: StateMachineSession,
    transitionResult: StateTransitionResult,
  ): void {
    // Update transition success rate
    if (transitionResult.success) {
      this.metrics.transitionSuccessRate =
        (this.metrics.transitionSuccessRate * this.metrics.totalTransitions +
          1) /
        (this.metrics.totalTransitions + 1);
    }

    // Update state timing
    const lastEntry = session.stateHistory[session.stateHistory.length - 1];
    if (lastEntry) {
      const timeInState = Date.now() - lastEntry.entryTime.getTime();
      const currentAvg =
        this.metrics.averageTimePerState[session.currentState] || 0;
      this.metrics.averageTimePerState[session.currentState] =
        (currentAvg + timeInState) / 2;
    }
  }

  /**
   * Get agent state for serialization
   */
  getState(): {
    sessions: Record<string, StateMachineSession>;
    metrics: StateMachineMetrics;
    config: StateMachineAgentConfig;
  } {
    const sessions: Record<string, StateMachineSession> = {};
    for (const [id, session] of this.sessions) {
      sessions[id] = session;
    }

    return {
      sessions,
      metrics: this.metrics,
      config: this.config,
    };
  }

  /**
   * Set agent state from serialization
   */
  async setState(state: {
    sessions: Record<string, StateMachineSession>;
    metrics: StateMachineMetrics;
    config: StateMachineAgentConfig;
  }): Promise<void> {
    this.sessions.clear();
    for (const [id, session] of Object.entries(state.sessions)) {
      this.sessions.set(id, session);
    }
    this.metrics = state.metrics;
    // Note: config is typically immutable after construction
  }

  /**
   * Clean up resources
   */
  async destroy(): Promise<void> {
    // Clear all timeouts
    for (const timeoutHandle of this.stateTimeouts.values()) {
      clearTimeout(timeoutHandle);
    }
    this.stateTimeouts.clear();

    // Mark all sessions as ended
    for (const session of this.sessions.values()) {
      if (session.status === "active") {
        session.status = "error";
      }
    }

    await super.destroy();
  }
}

/**
 * Factory function for creating state machine agents
 */
export function createStateMachineAgent(
  config: StateMachineAgentConfig & { name: string; description?: string },
  llmRouter: LLMRouter,
  logger?: Logger,
): StateMachineAgent {
  const agentDefinition: AgentDefinition = {
    id: `sm_agent_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    name: config.name,
    type: "state_machine",
    description: config.description,
    config: {
      llmProvider: "gpt-3.5-turbo",
      memory: { type: "both", maxSize: 1000 },
    },
    metadata: {
      createdAt: new Date().toISOString(),
      version: "1.0.0",
    },
  };

  return new StateMachineAgent(agentDefinition, config, llmRouter, logger);
}

/**
 * State machine agent templates for common workflows
 */
export const StateMachineTemplates = {
  /**
   * Order processing workflow
   */
  orderProcessing: (llmRouter: LLMRouter): StateMachineAgent => {
    return createStateMachineAgent(
      {
        name: "Order Processing Agent",
        description: "Handles customer order processing workflow",
        initialState: "greeting",
        states: {
          greeting: {
            name: "greeting",
            prompt:
              "Welcome! I can help you place an order. What would you like to purchase today?",
            availableTransitions: ["collecting_info", "existing_order"],
            onEntry: [],
            onExit: [],
          },
          collecting_info: {
            name: "collecting_info",
            prompt:
              "Please provide the following information: product name, quantity, and delivery address.",
            availableTransitions: ["validate_order", "request_clarification"],
            onEntry: [],
            onExit: [],
          },
          validate_order: {
            name: "validate_order",
            prompt:
              "Let me validate your order details. Please wait a moment...",
            availableTransitions: ["payment", "correction_needed"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "validateOrder" },
              },
            ],
            onExit: [],
          },
          payment: {
            name: "payment",
            prompt:
              "Your order is valid. Please provide payment information to proceed.",
            availableTransitions: ["processing", "payment_failed"],
            onEntry: [],
            onExit: [],
          },
          processing: {
            name: "processing",
            prompt: "Processing your order... This may take a few moments.",
            availableTransitions: ["completed", "error"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "processOrder" },
              },
            ],
            onExit: [],
          },
          completed: {
            name: "completed",
            prompt:
              "Your order has been successfully processed! You will receive a confirmation email shortly.",
            defaultResponse: "Thank you for your order!",
            availableTransitions: [],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "sendConfirmation" },
              },
            ],
            onExit: [],
          },
          error: {
            name: "error",
            prompt:
              "I apologize, but there was an error processing your order. Let me help you resolve this.",
            availableTransitions: ["collecting_info", "completed"],
            onEntry: [],
            onExit: [],
          },
        },
        transitions: [
          {
            fromState: "greeting",
            toState: "collecting_info",
            trigger: "order",
          },
          {
            fromState: "greeting",
            toState: "existing_order",
            trigger: "existing",
          },
          {
            fromState: "collecting_info",
            toState: "validate_order",
            condition: "hasAllInfo == true",
          },
          {
            fromState: "collecting_info",
            toState: "request_clarification",
            condition: "hasAllInfo == false",
          },
          {
            fromState: "validate_order",
            toState: "payment",
            condition: 'validationResult == "valid"',
          },
          {
            fromState: "validate_order",
            toState: "correction_needed",
            condition: 'validationResult == "invalid"',
          },
          {
            fromState: "payment",
            toState: "processing",
            trigger: "payment_confirmed",
          },
          {
            fromState: "payment",
            toState: "payment_failed",
            trigger: "payment_declined",
          },
          {
            fromState: "processing",
            toState: "completed",
            condition: 'orderStatus == "success"',
          },
          {
            fromState: "processing",
            toState: "error",
            condition: 'orderStatus == "failed"',
          },
        ],
        variables: {
          hasAllInfo: false,
          validationResult: "",
          orderStatus: "",
          orderId: "",
        },
        errorHandling: {
          fallbackState: "error",
          maxRetries: 3,
        },
      },
      llmRouter,
    );
  },

  /**
   * Support ticket workflow
   */
  supportTicket: (llmRouter: LLMRouter): StateMachineAgent => {
    return createStateMachineAgent(
      {
        name: "Support Ticket Agent",
        description: "Handles customer support ticket workflow",
        initialState: "initial_contact",
        states: {
          initial_contact: {
            name: "initial_contact",
            prompt:
              "Hello! I'm here to help with your support request. Please describe the issue you're experiencing.",
            availableTransitions: ["categorizing", "escalate"],
            onEntry: [],
            onExit: [],
          },
          categorizing: {
            name: "categorizing",
            prompt:
              "Let me categorize your issue to route it to the right specialist.",
            availableTransitions: [
              "technical_support",
              "billing_support",
              "general_inquiry",
            ],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "categorizeIssue" },
              },
            ],
            onExit: [],
          },
          technical_support: {
            name: "technical_support",
            prompt:
              "I'll help you with this technical issue. Let's start with some diagnostic questions.",
            availableTransitions: ["troubleshooting", "escalate_technical"],
            onEntry: [],
            onExit: [],
          },
          troubleshooting: {
            name: "troubleshooting",
            prompt:
              "Let's try these troubleshooting steps. Please follow the instructions carefully.",
            availableTransitions: [
              "resolved",
              "escalate_technical",
              "additional_steps",
            ],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "provideTroubleshootingSteps" },
              },
            ],
            onExit: [],
          },
          resolved: {
            name: "resolved",
            prompt:
              "Great! It looks like we've resolved your issue. Is there anything else I can help you with?",
            availableTransitions: ["closed", "new_issue"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "markResolved" },
              },
            ],
            onExit: [],
          },
          escalate_technical: {
            name: "escalate_technical",
            prompt:
              "I'm escalating your issue to our technical specialists. You'll receive an update within 24 hours.",
            availableTransitions: ["closed"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "escalateToTechnical" },
              },
            ],
            onExit: [],
          },
          closed: {
            name: "closed",
            prompt:
              "Your support ticket has been closed. Thank you for contacting us!",
            defaultResponse: "Thank you for using our support!",
            availableTransitions: [],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "closeTicket" },
              },
            ],
            onExit: [],
          },
        },
        transitions: [
          {
            fromState: "initial_contact",
            toState: "categorizing",
            condition: "issueDescription.length > 10",
          },
          {
            fromState: "initial_contact",
            toState: "escalate",
            trigger: "urgent",
          },
          {
            fromState: "categorizing",
            toState: "technical_support",
            condition: 'category == "technical"',
          },
          {
            fromState: "categorizing",
            toState: "billing_support",
            condition: 'category == "billing"',
          },
          {
            fromState: "technical_support",
            toState: "troubleshooting",
            trigger: "start_troubleshooting",
          },
          {
            fromState: "troubleshooting",
            toState: "resolved",
            trigger: "fixed",
          },
          {
            fromState: "troubleshooting",
            toState: "escalate_technical",
            trigger: "escalate",
          },
          { fromState: "resolved", toState: "closed", trigger: "done" },
          { fromState: "resolved", toState: "new_issue", trigger: "new_issue" },
        ],
        variables: {
          category: "",
          issueDescription: "",
          priority: "normal",
          ticketId: "",
        },
        timeouts: {
          initial_contact: 300, // 5 minutes
          categorizing: 60, // 1 minute
          troubleshooting: 600, // 10 minutes
        },
      },
      llmRouter,
    );
  },

  /**
   * Approval workflow
   */
  approvalWorkflow: (llmRouter: LLMRouter): StateMachineAgent => {
    return createStateMachineAgent(
      {
        name: "Approval Workflow Agent",
        description: "Manages approval requests and routing",
        initialState: "request_submitted",
        states: {
          request_submitted: {
            name: "request_submitted",
            prompt:
              "I've received your approval request. Let me review the details and determine the approval path.",
            availableTransitions: [
              "auto_approved",
              "manager_review",
              "additional_info_needed",
            ],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "analyzeRequest" },
              },
            ],
            onExit: [],
          },
          auto_approved: {
            name: "auto_approved",
            prompt:
              "Your request has been automatically approved! Processing will begin immediately.",
            availableTransitions: ["completed"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "processApproval" },
              },
            ],
            onExit: [],
          },
          manager_review: {
            name: "manager_review",
            prompt:
              "Your request requires manager approval. I've forwarded it to the appropriate manager.",
            availableTransitions: ["approved", "rejected", "escalated"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "notifyManager" },
              },
            ],
            onExit: [],
          },
          approved: {
            name: "approved",
            prompt: "Congratulations! Your request has been approved.",
            availableTransitions: ["completed"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "processApproval" },
              },
            ],
            onExit: [],
          },
          rejected: {
            name: "rejected",
            prompt:
              "I'm sorry, but your request has been rejected. Here's the feedback from the reviewer.",
            availableTransitions: ["resubmit", "appeal"],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "sendRejectionNotice" },
              },
            ],
            onExit: [],
          },
          completed: {
            name: "completed",
            prompt:
              "Your approval process is complete. All necessary actions have been taken.",
            defaultResponse: "Process completed successfully!",
            availableTransitions: [],
            onEntry: [
              {
                type: "function_call",
                config: { functionName: "finalizeProcess" },
              },
            ],
            onExit: [],
          },
        },
        transitions: [
          {
            fromState: "request_submitted",
            toState: "auto_approved",
            condition: "amount < 1000",
          },
          {
            fromState: "request_submitted",
            toState: "manager_review",
            condition: "amount >= 1000 && amount < 10000",
          },
          {
            fromState: "request_submitted",
            toState: "escalated",
            condition: "amount >= 10000",
          },
          {
            fromState: "auto_approved",
            toState: "completed",
            trigger: "processing_complete",
          },
          {
            fromState: "manager_review",
            toState: "approved",
            trigger: "manager_approved",
          },
          {
            fromState: "manager_review",
            toState: "rejected",
            trigger: "manager_rejected",
          },
          {
            fromState: "approved",
            toState: "completed",
            trigger: "processing_complete",
          },
          {
            fromState: "rejected",
            toState: "request_submitted",
            trigger: "resubmit",
          },
        ],
        variables: {
          amount: 0,
          requestType: "",
          requesterId: "",
          managerId: "",
          reviewComments: "",
        },
      },
      llmRouter,
    );
  },
};
