/**
 * Workflow Builder for Circuit Breaker TypeScript SDK
 *
 * This file provides a fluent API for building complex workflows with
 * state transitions, rules, conditions, and advanced patterns like
 * branching, parallel execution, and loops.
 */

import {
  WorkflowDefinition,
  ActivityDefinition,
  Rule,
  FunctionTrigger,
} from '../core/types.js';
import {
  WorkflowValidationError,
  ValidationError,
  RuleValidationError,
} from '../core/errors.js';
import { RulesEngine } from '../rules/engine.js';

// ============================================================================
// Types
// ============================================================================

export interface WorkflowValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
  stateCount: number;
  activityCount: number;
  ruleCount: number;
}

export interface BranchCondition {
  condition: string;
  targetState: string;
  activityId: string;
}

export interface ParallelBranch {
  name: string;
  states: string[];
  activities: ActivityDefinition[];
  joinState: string;
}

export interface LoopCondition {
  condition: string;
  maxIterations?: number;
  exitCondition?: string;
}

// ============================================================================
// Main Workflow Builder
// ============================================================================

export class WorkflowBuilder {
  private workflow: WorkflowDefinition;
  private rulesEngine?: RulesEngine;
  private stateIndex: number = 0;
  private activityIndex: number = 0;

  constructor(name: string) {
    this.workflow = {
      name,
      states: [],
      activities: [],
      initialState: '',
      metadata: {},
    };
  }

  // ============================================================================
  // Basic Building Methods
  // ============================================================================

  /**
   * Add a state to the workflow
   */
  addState(state: string): WorkflowBuilder {
    if (this.workflow.states.includes(state)) {
      throw new ValidationError(
        'state',
        state,
        'State already exists in workflow'
      );
    }

    this.workflow.states.push(state);
    return this;
  }

  /**
   * Add multiple states at once
   */
  addStates(states: string[]): WorkflowBuilder {
    states.forEach(state => this.addState(state));
    return this;
  }

  /**
   * Add a transition between states
   */
  addTransition(
    from: string,
    to: string,
    activityId: string,
    options?: {
      name?: string;
      description?: string;
      conditions?: string[];
      rules?: Rule[];
      functions?: FunctionTrigger[];
      requiresAllRules?: boolean;
      metadata?: Record<string, any>;
    }
  ): WorkflowBuilder {
    // Ensure states exist
    if (!this.workflow.states.includes(from)) {
      this.addState(from);
    }
    if (!this.workflow.states.includes(to)) {
      this.addState(to);
    }

    // Check if activity already exists
    const existingActivity = this.workflow.activities.find(a => a.id === activityId);
    if (existingActivity) {
      throw new ValidationError(
        'activityId',
        activityId,
        'Activity ID already exists in workflow'
      );
    }

    const activity: ActivityDefinition = {
      id: activityId,
      name: options?.name,
      fromStates: [from],
      toState: to,
      conditions: options?.conditions || [],
      rules: options?.rules,
      functions: options?.functions,
      requiresAllRules: options?.requiresAllRules,
      description: options?.description,
      metadata: options?.metadata,
    };

    this.workflow.activities.push(activity);
    return this;
  }

  /**
   * Add a rule to a specific activity
   */
  addRule(activityId: string, rule: Rule): WorkflowBuilder {
    const activity = this.findActivity(activityId);
    if (!activity) {
      throw new ValidationError(
        'activityId',
        activityId,
        'Activity not found in workflow'
      );
    }

    if (!activity.rules) {
      activity.rules = [];
    }

    activity.rules.push(rule);
    return this;
  }

  /**
   * Add multiple rules to a specific activity
   */
  addRules(
    activityId: string,
    rules: Rule[],
    requireAll: boolean = true
  ): WorkflowBuilder {
    const activity = this.findActivity(activityId);
    if (!activity) {
      throw new ValidationError(
        'activityId',
        activityId,
        'Activity not found in workflow'
      );
    }

    if (!activity.rules) {
      activity.rules = [];
    }

    activity.rules.push(...rules);
    activity.requiresAllRules = requireAll;
    return this;
  }

  /**
   * Add a simple rule using field comparison
   */
  addSimpleRule(
    activityId: string,
    field: string,
    operator: '==' | '!=' | '>' | '<' | '>=' | '<=' | 'contains' | 'exists',
    value?: any
  ): WorkflowBuilder {
    let condition: string;

    switch (operator) {
      case 'exists':
        condition = `data.${field} != null`;
        break;
      case 'contains':
        condition = `data.${field}.includes('${value}')`;
        break;
      default:
        const serializedValue = typeof value === 'string' ? `'${value}'` : value;
        condition = `data.${field} ${operator} ${serializedValue}`;
    }

    const rule: Rule = {
      name: `${field}_${operator}_rule`,
      type: 'simple',
      condition,
      description: `Check if ${field} ${operator} ${value || 'exists'}`,
    };

    return this.addRule(activityId, rule);
  }

  /**
   * Add a condition (legacy string-based) to an activity
   */
  addCondition(activityId: string, condition: string): WorkflowBuilder {
    const activity = this.findActivity(activityId);
    if (!activity) {
      throw new ValidationError(
        'activityId',
        activityId,
        'Activity not found in workflow'
      );
    }

    activity.conditions.push(condition);
    return this;
  }

  /**
   * Add a function trigger to an activity
   */
  addFunctionTrigger(
    activityId: string,
    functionTrigger: FunctionTrigger
  ): WorkflowBuilder {
    const activity = this.findActivity(activityId);
    if (!activity) {
      throw new ValidationError(
        'activityId',
        activityId,
        'Activity not found in workflow'
      );
    }

    if (!activity.functions) {
      activity.functions = [];
    }

    activity.functions.push(functionTrigger);
    return this;
  }

  /**
   * Set the initial state
   */
  setInitialState(state: string): WorkflowBuilder {
    if (!this.workflow.states.includes(state)) {
      this.addState(state);
    }

    this.workflow.initialState = state;
    return this;
  }

  /**
   * Add metadata to the workflow
   */
  addMetadata(key: string, value: any): WorkflowBuilder {
    this.workflow.metadata![key] = value;
    return this;
  }

  /**
   * Set workflow description
   */
  setDescription(description: string): WorkflowBuilder {
    this.workflow.description = description;
    return this;
  }

  /**
   * Set workflow version
   */
  setVersion(version: string): WorkflowBuilder {
    this.workflow.version = version;
    return this;
  }

  /**
   * Add workflow tags
   */
  addTags(tags: string[]): WorkflowBuilder {
    if (!this.workflow.tags) {
      this.workflow.tags = [];
    }
    this.workflow.tags.push(...tags);
    return this;
  }

  // ============================================================================
  // Advanced Workflow Patterns
  // ============================================================================

  /**
   * Create a branching pattern (decision point)
   */
  branch(fromState: string, conditions: BranchCondition[]): BranchBuilder {
    return new BranchBuilder(this, fromState, conditions);
  }

  /**
   * Create a parallel execution pattern
   */
  parallel(fromState: string, branches: ParallelBranch[]): ParallelBuilder {
    return new ParallelBuilder(this, fromState, branches);
  }

  /**
   * Create a loop pattern
   */
  loop(
    entryState: string,
    loopStates: string[],
    condition: LoopCondition
  ): LoopBuilder {
    return new LoopBuilder(this, entryState, loopStates, condition);
  }

  /**
   * Create a try-catch pattern for error handling
   */
  tryCatch(
    tryStates: string[],
    catchState: string,
    finallyState?: string
  ): TryCatchBuilder {
    return new TryCatchBuilder(this, tryStates, catchState, finallyState);
  }

  // ============================================================================
  // Rules Engine Integration
  // ============================================================================

  /**
   * Associate a rules engine for validation
   */
  withRulesEngine(rulesEngine: RulesEngine): WorkflowBuilder {
    this.rulesEngine = rulesEngine;
    return this;
  }

  /**
   * Validate rules in the workflow
   */
  async validateRules(): Promise<WorkflowValidationResult> {
    if (!this.rulesEngine) {
      return {
        valid: true,
        errors: [],
        warnings: ['No rules engine provided for validation'],
        stateCount: this.workflow.states.length,
        activityCount: this.workflow.activities.length,
        ruleCount: this.countRules(),
      };
    }

    try {
      const result = await this.rulesEngine.validateWorkflow(this.workflow);
      return {
        valid: result.valid,
        errors: result.errors,
        warnings: result.warnings,
        stateCount: this.workflow.states.length,
        activityCount: this.workflow.activities.length,
        ruleCount: this.countRules(),
      };
    } catch (error) {
      return {
        valid: false,
        errors: [`Rule validation failed: ${error}`],
        warnings: [],
        stateCount: this.workflow.states.length,
        activityCount: this.workflow.activities.length,
        ruleCount: this.countRules(),
      };
    }
  }

  // ============================================================================
  // Validation and Building
  // ============================================================================

  /**
   * Validate the workflow structure
   */
  validate(): WorkflowValidationResult {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check basic requirements
    if (!this.workflow.name) {
      errors.push('Workflow name is required');
    }

    if (this.workflow.states.length === 0) {
      errors.push('Workflow must have at least one state');
    }

    if (!this.workflow.initialState) {
      errors.push('Initial state must be specified');
    } else if (!this.workflow.states.includes(this.workflow.initialState)) {
      errors.push('Initial state must be one of the defined states');
    }

    // Validate states
    const stateNames = new Set<string>();
    for (const state of this.workflow.states) {
      if (stateNames.has(state)) {
        errors.push(`Duplicate state: ${state}`);
      }
      stateNames.add(state);

      if (!state || state.trim() === '') {
        errors.push('State names cannot be empty');
      }
    }

    // Validate activities
    const activityIds = new Set<string>();
    for (const activity of this.workflow.activities) {
      if (activityIds.has(activity.id)) {
        errors.push(`Duplicate activity ID: ${activity.id}`);
      }
      activityIds.add(activity.id);

      if (!activity.id || activity.id.trim() === '') {
        errors.push('Activity IDs cannot be empty');
      }

      // Check from states
      for (const fromState of activity.fromStates) {
        if (!this.workflow.states.includes(fromState)) {
          errors.push(`Activity ${activity.id} references unknown from state: ${fromState}`);
        }
      }

      // Check to state
      if (!this.workflow.states.includes(activity.toState)) {
        errors.push(`Activity ${activity.id} references unknown to state: ${activity.toState}`);
      }

      // Check for self-transitions
      if (activity.fromStates.includes(activity.toState)) {
        warnings.push(`Activity ${activity.id} creates a self-transition`);
      }
    }

    // Check for unreachable states
    const reachableStates = new Set([this.workflow.initialState]);
    let changed = true;
    while (changed) {
      changed = false;
      for (const activity of this.workflow.activities) {
        if (activity.fromStates.some(state => reachableStates.has(state))) {
          if (!reachableStates.has(activity.toState)) {
            reachableStates.add(activity.toState);
            changed = true;
          }
        }
      }
    }

    for (const state of this.workflow.states) {
      if (!reachableStates.has(state)) {
        warnings.push(`State ${state} is not reachable from the initial state`);
      }
    }

    // Check for terminal states (states with no outgoing transitions)
    const statesWithOutgoing = new Set<string>();
    for (const activity of this.workflow.activities) {
      activity.fromStates.forEach(state => statesWithOutgoing.add(state));
    }

    const terminalStates = this.workflow.states.filter(state => !statesWithOutgoing.has(state));
    if (terminalStates.length === 0) {
      warnings.push('Workflow has no terminal states (all states have outgoing transitions)');
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      stateCount: this.workflow.states.length,
      activityCount: this.workflow.activities.length,
      ruleCount: this.countRules(),
    };
  }

  /**
   * Build and return the workflow definition
   */
  build(): WorkflowDefinition {
    const validation = this.validate();
    if (!validation.valid) {
      throw new WorkflowValidationError(validation.errors);
    }

    return { ...this.workflow };
  }

  /**
   * Build workflow with async rule validation
   */
  async buildAsync(): Promise<WorkflowDefinition> {
    const structuralValidation = this.validate();
    if (!structuralValidation.valid) {
      throw new WorkflowValidationError(structuralValidation.errors);
    }

    if (this.rulesEngine) {
      const ruleValidation = await this.validateRules();
      if (!ruleValidation.valid) {
        throw new RuleValidationError(ruleValidation.errors);
      }
    }

    return { ...this.workflow };
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  /**
   * Get current workflow definition (for inspection)
   */
  getWorkflow(): Readonly<WorkflowDefinition> {
    return { ...this.workflow };
  }

  /**
   * Clone the builder
   */
  clone(): WorkflowBuilder {
    const newBuilder = new WorkflowBuilder(this.workflow.name);
    newBuilder.workflow = JSON.parse(JSON.stringify(this.workflow));
    newBuilder.rulesEngine = this.rulesEngine;
    return newBuilder;
  }

  /**
   * Reset the builder
   */
  reset(): WorkflowBuilder {
    this.workflow = {
      name: this.workflow.name,
      states: [],
      activities: [],
      initialState: '',
      metadata: {},
    };
    return this;
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private findActivity(activityId: string): ActivityDefinition | undefined {
    return this.workflow.activities.find(a => a.id === activityId);
  }

  private generateStateId(): string {
    return `state_${++this.stateIndex}`;
  }

  private generateActivityId(): string {
    return `activity_${++this.activityIndex}`;
  }

  private countRules(): number {
    return this.workflow.activities.reduce(
      (count, activity) => count + (activity.rules?.length || 0),
      0
    );
  }
}

// ============================================================================
// Advanced Pattern Builders
// ============================================================================

export class BranchBuilder {
  constructor(
    private parent: WorkflowBuilder,
    private fromState: string,
    private conditions: BranchCondition[]
  ) {}

  /**
   * Add a conditional branch
   */
  when(condition: string, targetState: string, activityId?: string): BranchBuilder {
    const id = activityId || `branch_${this.conditions.length}`;
    this.conditions.push({ condition, targetState, activityId: id });
    return this;
  }

  /**
   * Add an else/default branch
   */
  otherwise(targetState: string, activityId?: string): WorkflowBuilder {
    const id = activityId || 'branch_default';

    // Add all conditional branches
    this.conditions.forEach(branch => {
      this.parent.addTransition(this.fromState, branch.targetState, branch.activityId, {
        conditions: [branch.condition],
      });
    });

    // Add default branch (with negated conditions)
    const negatedConditions = this.conditions.map(c => `!(${c.condition})`);
    this.parent.addTransition(this.fromState, targetState, id, {
      conditions: negatedConditions.length > 0 ? [negatedConditions.join(' && ')] : [],
    });

    return this.parent;
  }

  /**
   * Complete branching without a default case
   */
  done(): WorkflowBuilder {
    this.conditions.forEach(branch => {
      this.parent.addTransition(this.fromState, branch.targetState, branch.activityId, {
        conditions: [branch.condition],
      });
    });

    return this.parent;
  }
}

export class ParallelBuilder {
  constructor(
    private parent: WorkflowBuilder,
    private fromState: string,
    private branches: ParallelBranch[]
  ) {}

  /**
   * Add a parallel branch
   */
  addBranch(branch: ParallelBranch): ParallelBuilder {
    this.branches.push(branch);
    return this;
  }

  /**
   * Complete parallel pattern
   */
  joinAt(joinState: string): WorkflowBuilder {
    // Add all parallel branches
    this.branches.forEach(branch => {
      // Add states for this branch
      branch.states.forEach(state => this.parent.addState(state));

      // Add activities for this branch
      branch.activities.forEach(activity => {
        this.parent.workflow.activities.push(activity);
      });

      // Add fork transition
      this.parent.addTransition(
        this.fromState,
        branch.states[0],
        `fork_${branch.name}`,
        { name: `Fork to ${branch.name}` }
      );

      // Add join transition
      this.parent.addTransition(
        branch.joinState || branch.states[branch.states.length - 1],
        joinState,
        `join_${branch.name}`,
        { name: `Join from ${branch.name}` }
      );
    });

    return this.parent;
  }
}

export class LoopBuilder {
  constructor(
    private parent: WorkflowBuilder,
    private entryState: string,
    private loopStates: string[],
    private condition: LoopCondition
  ) {}

  /**
   * Set loop body states and transitions
   */
  withBody(activities: ActivityDefinition[]): LoopBuilder {
    activities.forEach(activity => {
      this.parent.workflow.activities.push(activity);
    });
    return this;
  }

  /**
   * Set exit state for the loop
   */
  exitTo(exitState: string): WorkflowBuilder {
    // Add loop continuation condition
    this.parent.addTransition(
      this.loopStates[this.loopStates.length - 1],
      this.entryState,
      'loop_continue',
      {
        conditions: [this.condition.condition],
        name: 'Continue Loop',
      }
    );

    // Add loop exit condition
    const exitCondition = this.condition.exitCondition || `!(${this.condition.condition})`;
    this.parent.addTransition(
      this.loopStates[this.loopStates.length - 1],
      exitState,
      'loop_exit',
      {
        conditions: [exitCondition],
        name: 'Exit Loop',
      }
    );

    return this.parent;
  }
}

export class TryCatchBuilder {
  constructor(
    private parent: WorkflowBuilder,
    private tryStates: string[],
    private catchState: string,
    private finallyState?: string
  ) {}

  /**
   * Add error handling transitions
   */
  onError(errorCondition: string): TryCatchBuilder {
    // Add error transitions from all try states to catch state
    this.tryStates.forEach(state => {
      this.parent.addTransition(state, this.catchState, `error_${state}`, {
        conditions: [errorCondition],
        name: 'Handle Error',
      });
    });

    return this;
  }

  /**
   * Complete try-catch pattern
   */
  done(): WorkflowBuilder {
    if (this.finallyState) {
      // Add transitions to finally state
      this.tryStates.forEach(state => {
        this.parent.addTransition(state, this.finallyState!, `finally_${state}`, {
          name: 'Finally Block',
        });
      });

      this.parent.addTransition(this.catchState, this.finallyState, 'finally_catch', {
        name: 'Finally from Catch',
      });
    }

    return this.parent;
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Create a new workflow builder
 */
export function createWorkflow(name: string): WorkflowBuilder {
  return new WorkflowBuilder(name);
}

/**
 * Create a simple linear workflow
 */
export function createLinearWorkflow(
  name: string,
  states: string[],
  activities?: string[]
): WorkflowBuilder {
  const builder = new WorkflowBuilder(name);

  builder.addStates(states);
  builder.setInitialState(states[0]);

  for (let i = 0; i < states.length - 1; i++) {
    const activityId = activities?.[i] || `step_${i + 1}`;
    builder.addTransition(states[i], states[i + 1], activityId);
  }

  return builder;
}

/**
 * Create a workflow from a state machine definition
 */
export function createFromStateMachine(definition: {
  name: string;
  states: string[];
  transitions: Array<{
    from: string;
    to: string;
    event: string;
    condition?: string;
  }>;
  initialState: string;
}): WorkflowBuilder {
  const builder = new WorkflowBuilder(definition.name);

  builder.addStates(definition.states);
  builder.setInitialState(definition.initialState);

  definition.transitions.forEach(transition => {
    builder.addTransition(transition.from, transition.to, transition.event, {
      conditions: transition.condition ? [transition.condition] : [],
    });
  });

  return builder;
}
