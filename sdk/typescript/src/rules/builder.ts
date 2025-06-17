/**
 * Rule Builder for Circuit Breaker TypeScript SDK
 *
 * This file provides a fluent interface for creating and managing rules
 * with advanced features like validation, composition, and testing.
 */

import {
  Rule,
  RuleType,
  RuleContext,
  RuleEvaluator,
  RuleEvaluationResult,
} from "../core/types.js";
import { RuleValidationError } from "../core/errors.js";
import { RulesEngine, RuleCreateInput } from "./engine.js";

// ============================================================================
// Types
// ============================================================================

export interface RuleBuilderOptions {
  /** Whether to validate rules during building */
  validate?: boolean;

  /** Default category for rules */
  defaultCategory?: string;

  /** Default priority for rules */
  defaultPriority?: number;

  /** Default metadata to apply to all rules */
  defaultMetadata?: Record<string, any>;

  /** Whether to auto-enable rules */
  autoEnable?: boolean;
}

export interface RuleTemplate {
  /** Template name */
  name: string;

  /** Template description */
  description?: string;

  /** Rule type */
  type: RuleType;

  /** Template condition (with placeholders) */
  condition?: string;

  /** Template category */
  category?: string;

  /** Template metadata */
  metadata?: Record<string, any>;

  /** Template parameters */
  parameters?: string[];
}

export interface ConditionalRuleGroup {
  /** Group name */
  name: string;

  /** Rules in the group */
  rules: Rule[];

  /** Condition to enable the group */
  enableCondition?: (context: RuleContext) => boolean | Promise<boolean>;

  /** Group metadata */
  metadata?: Record<string, any>;
}

export interface RuleChain {
  /** Chain name */
  name: string;

  /** Rules in execution order */
  rules: Rule[];

  /** Whether to stop on first failure */
  stopOnFailure?: boolean;

  /** Chain-wide context transformer */
  contextTransformer?: (context: RuleContext, ruleIndex: number) => RuleContext;
}

// ============================================================================
// Rule Builder
// ============================================================================

export class RuleBuilder {
  private readonly rulesEngine: RulesEngine;
  private readonly options: RuleBuilderOptions;
  private readonly templates = new Map<string, RuleTemplate>();
  private readonly conditionalGroups = new Map<string, ConditionalRuleGroup>();
  private readonly chains = new Map<string, RuleChain>();

  constructor(rulesEngine: RulesEngine, options: RuleBuilderOptions = {}) {
    this.rulesEngine = rulesEngine;
    this.options = {
      validate: true,
      autoEnable: true,
      defaultPriority: 0,
      ...options,
    };
  }

  // ============================================================================
  // Basic Rule Creation
  // ============================================================================

  /**
   * Start building a simple rule
   */
  simple(name: string): SimpleRuleBuilder {
    return new SimpleRuleBuilder(name, this.rulesEngine, this.options);
  }

  /**
   * Start building a JavaScript rule
   */
  javascript(name: string): JavaScriptRuleBuilder {
    return new JavaScriptRuleBuilder(name, this.rulesEngine, this.options);
  }

  /**
   * Start building a composite rule
   */
  composite(name: string): CompositeRuleBuilder {
    return new CompositeRuleBuilder(name, this.rulesEngine, this.options);
  }

  /**
   * Start building a custom rule
   */
  custom(name: string): CustomRuleBuilder {
    return new CustomRuleBuilder(name, this.rulesEngine, this.options);
  }

  // ============================================================================
  // Template Management
  // ============================================================================

  /**
   * Register a rule template
   */
  registerTemplate(template: RuleTemplate): this {
    this.templates.set(template.name, template);
    return this;
  }

  /**
   * Get a registered template
   */
  getTemplate(name: string): RuleTemplate | undefined {
    return this.templates.get(name);
  }

  /**
   * Create a rule from template
   */
  fromTemplate(
    templateName: string,
    name: string,
    parameters: Record<string, any> = {},
  ): RuleBuilderResult {
    const template = this.templates.get(templateName);
    if (!template) {
      throw new RuleValidationError([`Template not found: ${templateName}`], {
        templateName,
      });
    }

    let condition = template.condition || "";

    // Replace parameters in condition
    if (template.parameters) {
      for (const param of template.parameters) {
        if (parameters[param] === undefined) {
          throw new RuleValidationError(
            [`Missing required parameter: ${param}`],
            { templateName, parameters: template.parameters },
          );
        }
        condition = condition.replace(
          new RegExp(`\\{\\{${param}\\}\\}`, "g"),
          parameters[param],
        );
      }
    }

    const ruleInput: RuleCreateInput = {
      name,
      type: template.type,
      condition,
      ...(template.description ? { description: template.description } : {}),
      ...(template.category || this.options.defaultCategory
        ? { category: template.category || this.options.defaultCategory }
        : {}),
      metadata: {
        ...this.options.defaultMetadata,
        ...template.metadata,
        templateName,
        templateParameters: parameters,
      },
      ...(this.options.defaultPriority !== undefined
        ? { priority: this.options.defaultPriority }
        : {}),
      ...(this.options.autoEnable !== undefined
        ? { enabled: this.options.autoEnable }
        : {}),
    };

    return new RuleBuilderResult(ruleInput, this.rulesEngine);
  }

  // ============================================================================
  // Conditional Groups
  // ============================================================================

  /**
   * Create a conditional rule group
   */
  conditionalGroup(name: string): ConditionalGroupBuilder {
    return new ConditionalGroupBuilder(name, this);
  }

  /**
   * Register a conditional group
   */
  registerConditionalGroup(group: ConditionalRuleGroup): this {
    this.conditionalGroups.set(group.name, group);
    return this;
  }

  /**
   * Evaluate a conditional group
   */
  async evaluateConditionalGroup(
    groupName: string,
    context: RuleContext,
  ): Promise<RuleEvaluationResult | null> {
    const group = this.conditionalGroups.get(groupName);
    if (!group) {
      throw new RuleValidationError(
        [`Conditional group not found: ${groupName}`],
        { groupName },
      );
    }

    // Check if group should be enabled
    if (group.enableCondition) {
      const shouldEnable = await group.enableCondition(context);
      if (!shouldEnable) {
        return null;
      }
    }

    // Evaluate all rules in the group
    const ruleNames = group.rules.map((r) => r.name);
    return this.rulesEngine.evaluateRules(ruleNames, context);
  }

  // ============================================================================
  // Rule Chains
  // ============================================================================

  /**
   * Create a rule chain
   */
  chain(name: string): RuleChainBuilder {
    return new RuleChainBuilder(name, this);
  }

  /**
   * Register a rule chain
   */
  registerChain(chain: RuleChain): this {
    this.chains.set(chain.name, chain);
    return this;
  }

  /**
   * Execute a rule chain
   */
  async executeChain(
    chainName: string,
    context: RuleContext,
  ): Promise<RuleEvaluationResult> {
    const chain = this.chains.get(chainName);
    if (!chain) {
      throw new RuleValidationError([`Rule chain not found: ${chainName}`], {
        chainName,
      });
    }

    const results: any[] = [];
    const errors: string[] = [];
    let currentContext = context;

    for (let i = 0; i < chain.rules.length; i++) {
      const rule = chain.rules[i];
      if (!rule) continue;

      try {
        // Transform context if transformer is provided
        if (chain.contextTransformer) {
          currentContext = chain.contextTransformer(currentContext, i);
        }

        const result = await this.rulesEngine.evaluateRule(
          rule.name || "",
          currentContext,
        );

        results.push(...result.results);
        errors.push(...result.errors);

        // Stop on failure if configured
        if (chain.stopOnFailure && !result.passed) {
          break;
        }
      } catch (error) {
        const errorMsg = `Chain rule '${rule.name}' failed: ${error}`;
        errors.push(errorMsg);

        if (chain.stopOnFailure) {
          break;
        }
      }
    }

    return {
      passed: errors.length === 0 && results.every((r) => r.passed),
      results,
      errors,
      evaluationTime: results.reduce(
        (sum, r) => sum + (r.executionTime || 0),
        0,
      ),
      rulesEvaluated: results.length,
      rulesPassed: results.filter((r) => r.passed).length,
    };
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  /**
   * Create multiple rules with common properties
   */
  createBatch(
    rules: Array<{
      name: string;
      type: RuleType;
      condition?: string;
      evaluator?: RuleEvaluator;
    }>,
    commonOptions: Partial<RuleCreateInput> = {},
  ): BatchRuleBuilder {
    return new BatchRuleBuilder(
      rules,
      commonOptions,
      this.rulesEngine,
      this.options,
    );
  }

  /**
   * Clone an existing rule with modifications
   */
  async clone(
    sourceRuleName: string,
    newName: string,
    modifications: Partial<RuleCreateInput> = {},
  ): Promise<RuleBuilderResult> {
    const sourceRule = await this.rulesEngine.get(sourceRuleName);

    const ruleInput: RuleCreateInput = {
      name: newName,
      type: sourceRule.type,
      condition: sourceRule.condition,
      description: sourceRule.description,
      category: sourceRule.category,
      metadata: { ...sourceRule.metadata, clonedFrom: sourceRuleName },
      priority: sourceRule.priority,
      enabled: sourceRule.enabled,
      ...modifications,
    };

    return new RuleBuilderResult(ruleInput, this.rulesEngine);
  }

  /**
   * Test a rule without saving it
   */
  async testRule(
    ruleInput: RuleCreateInput,
    context: RuleContext,
  ): Promise<RuleEvaluationResult> {
    // Create a temporary rule instance for testing
    const tempRule: Rule = {
      ...ruleInput,
      enabled: true,
    };

    // Use engine's internal evaluation logic
    try {
      return await this.rulesEngine.evaluateRule(tempRule.name, context);
    } catch (error) {
      return {
        passed: false,
        results: [
          {
            rule: tempRule,
            passed: false,
            error: error instanceof Error ? error.message : String(error),
          },
        ],
        errors: [error instanceof Error ? error.message : String(error)],
        evaluationTime: 0,
        rulesEvaluated: 1,
        rulesPassed: 0,
      };
    }
  }
}

// ============================================================================
// Specific Rule Builders
// ============================================================================

export class SimpleRuleBuilder {
  private ruleInput: Partial<RuleCreateInput>;

  constructor(
    name: string,
    private rulesEngine: RulesEngine,
    private options: RuleBuilderOptions,
  ) {
    this.ruleInput = {
      name,
      type: "simple",
      ...(this.options.autoEnable !== undefined
        ? { enabled: this.options.autoEnable }
        : {}),
      ...(this.options.defaultPriority !== undefined
        ? { priority: this.options.defaultPriority }
        : {}),
      ...(this.options.defaultCategory
        ? { category: this.options.defaultCategory }
        : {}),
      metadata: { ...this.options.defaultMetadata },
    };
  }

  condition(condition: string): this {
    this.ruleInput.condition = condition;
    return this;
  }

  description(description: string): this {
    this.ruleInput.description = description;
    return this;
  }

  category(category: string): this {
    this.ruleInput.category = category;
    return this;
  }

  priority(priority: number): this {
    this.ruleInput.priority = priority;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.ruleInput.metadata = { ...this.ruleInput.metadata, ...metadata };
    return this;
  }

  enabled(enabled: boolean): this {
    this.ruleInput.enabled = enabled;
    return this;
  }

  build(): RuleBuilderResult {
    if (!this.ruleInput.condition) {
      throw new RuleValidationError(
        ["Condition is required for simple rules"],
        { ruleInput: this.ruleInput },
      );
    }

    return new RuleBuilderResult(
      this.ruleInput as RuleCreateInput,
      this.rulesEngine,
    );
  }

  async create(): Promise<Rule> {
    return this.build().create();
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult> {
    return this.build().test(context);
  }
}

export class JavaScriptRuleBuilder {
  private ruleInput: Partial<RuleCreateInput>;

  constructor(
    name: string,
    private rulesEngine: RulesEngine,
    private options: RuleBuilderOptions,
  ) {
    this.ruleInput = {
      name,
      type: "javascript",
      ...(this.options.autoEnable !== undefined
        ? { enabled: this.options.autoEnable }
        : {}),
      ...(this.options.defaultPriority !== undefined
        ? { priority: this.options.defaultPriority }
        : {}),
      ...(this.options.defaultCategory
        ? { category: this.options.defaultCategory }
        : {}),
      metadata: { ...this.options.defaultMetadata },
    };
  }

  condition(condition: string): this {
    this.ruleInput.condition = condition;
    return this;
  }

  description(description: string): this {
    this.ruleInput.description = description;
    return this;
  }

  category(category: string): this {
    this.ruleInput.category = category;
    return this;
  }

  priority(priority: number): this {
    this.ruleInput.priority = priority;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.ruleInput.metadata = { ...this.ruleInput.metadata, ...metadata };
    return this;
  }

  enabled(enabled: boolean): this {
    this.ruleInput.enabled = enabled;
    return this;
  }

  build(): RuleBuilderResult {
    if (!this.ruleInput.condition) {
      throw new RuleValidationError(
        ["Condition is required for JavaScript rules"],
        { ruleInput: this.ruleInput },
      );
    }

    // Validate JavaScript syntax
    try {
      new Function("context", this.ruleInput.condition);
    } catch (error) {
      throw new RuleValidationError(
        [`Invalid JavaScript condition: ${error}`],
        { condition: this.ruleInput.condition },
      );
    }

    return new RuleBuilderResult(
      this.ruleInput as RuleCreateInput,
      this.rulesEngine,
    );
  }

  async create(): Promise<Rule> {
    return this.build().create();
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult> {
    return this.build().test(context);
  }
}

export class CompositeRuleBuilder {
  private ruleInput: Partial<RuleCreateInput>;
  private childRules: Rule[] = [];

  constructor(
    name: string,
    private rulesEngine: RulesEngine,
    private options: RuleBuilderOptions,
  ) {
    this.ruleInput = {
      name,
      type: "composite",
      ...(this.options.autoEnable !== undefined
        ? { enabled: this.options.autoEnable }
        : {}),
      ...(this.options.defaultPriority !== undefined
        ? { priority: this.options.defaultPriority }
        : {}),
      ...(this.options.defaultCategory
        ? { category: this.options.defaultCategory }
        : {}),
      metadata: { ...this.options.defaultMetadata },
    };
  }

  operator(operator: "AND" | "OR" | "NOT"): this {
    this.ruleInput.operator = operator;
    return this;
  }

  addRule(rule: Rule): this {
    this.childRules.push(rule);
    return this;
  }

  addRules(rules: Rule[]): this {
    this.childRules.push(...rules);
    return this;
  }

  addRuleByName(ruleName: string): this {
    // This will be resolved when the composite rule is evaluated
    this.childRules.push({ name: ruleName } as Rule);
    return this;
  }

  description(description: string): this {
    this.ruleInput.description = description;
    return this;
  }

  category(category: string): this {
    this.ruleInput.category = category;
    return this;
  }

  priority(priority: number): this {
    this.ruleInput.priority = priority;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.ruleInput.metadata = { ...this.ruleInput.metadata, ...metadata };
    return this;
  }

  enabled(enabled: boolean): this {
    this.ruleInput.enabled = enabled;
    return this;
  }

  build(): RuleBuilderResult {
    if (!this.ruleInput.operator) {
      throw new RuleValidationError(
        ["Operator is required for composite rules"],
        { ruleInput: this.ruleInput },
      );
    }

    if (this.childRules.length === 0) {
      throw new RuleValidationError(
        ["At least one child rule is required for composite rules"],
        { ruleInput: this.ruleInput },
      );
    }

    this.ruleInput.rules = this.childRules;

    return new RuleBuilderResult(
      this.ruleInput as RuleCreateInput,
      this.rulesEngine,
    );
  }

  async create(): Promise<Rule> {
    return this.build().create();
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult> {
    return this.build().test(context);
  }
}

export class CustomRuleBuilder {
  private ruleInput: Partial<RuleCreateInput>;

  constructor(
    name: string,
    private rulesEngine: RulesEngine,
    private options: RuleBuilderOptions,
  ) {
    this.ruleInput = {
      name,
      type: "custom",
      ...(this.options.autoEnable !== undefined
        ? { enabled: this.options.autoEnable }
        : {}),
      ...(this.options.defaultPriority !== undefined
        ? { priority: this.options.defaultPriority }
        : {}),
      ...(this.options.defaultCategory
        ? { category: this.options.defaultCategory }
        : {}),
      metadata: { ...this.options.defaultMetadata },
    };
  }

  evaluator(evaluator: RuleEvaluator): this {
    this.ruleInput.evaluator = evaluator;
    return this;
  }

  description(description: string): this {
    this.ruleInput.description = description;
    return this;
  }

  category(category: string): this {
    this.ruleInput.category = category;
    return this;
  }

  priority(priority: number): this {
    this.ruleInput.priority = priority;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.ruleInput.metadata = { ...this.ruleInput.metadata, ...metadata };
    return this;
  }

  enabled(enabled: boolean): this {
    this.ruleInput.enabled = enabled;
    return this;
  }

  build(): RuleBuilderResult {
    if (!this.ruleInput.evaluator) {
      throw new RuleValidationError(
        ["Evaluator is required for custom rules"],
        { ruleInput: this.ruleInput },
      );
    }

    return new RuleBuilderResult(
      this.ruleInput as RuleCreateInput,
      this.rulesEngine,
    );
  }

  async create(): Promise<Rule> {
    return this.build().create();
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult> {
    return this.build().test(context);
  }
}

// ============================================================================
// Builder Results and Utilities
// ============================================================================

export class RuleBuilderResult {
  constructor(
    private ruleInput: RuleCreateInput,
    private rulesEngine: RulesEngine,
  ) {}

  async create(): Promise<Rule> {
    return this.rulesEngine.create(this.ruleInput);
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult> {
    // Create a temporary rule for testing
    const tempRule: Rule = {
      ...this.ruleInput,
      enabled: true,
    };

    try {
      return await this.rulesEngine.evaluateRule(tempRule.name, context);
    } catch (error) {
      return {
        passed: false,
        results: [
          {
            rule: tempRule,
            passed: false,
            error: error instanceof Error ? error.message : String(error),
          },
        ],
        errors: [error instanceof Error ? error.message : String(error)],
        evaluationTime: 0,
        rulesEvaluated: 1,
        rulesPassed: 0,
      };
    }
  }

  getRuleInput(): RuleCreateInput {
    return { ...this.ruleInput };
  }
}

export class BatchRuleBuilder {
  constructor(
    private rules: Array<{
      name: string;
      type: RuleType;
      condition?: string;
      evaluator?: RuleEvaluator;
    }>,
    private commonOptions: Partial<RuleCreateInput>,
    private rulesEngine: RulesEngine,
    private options: RuleBuilderOptions,
  ) {}

  async create(): Promise<Rule[]> {
    const createdRules: Rule[] = [];

    for (const rule of this.rules) {
      const ruleInput: RuleCreateInput = {
        ...rule,
        ...this.commonOptions,
        ...(this.options.autoEnable !== undefined
          ? { enabled: this.options.autoEnable }
          : {}),
        ...(this.options.defaultPriority !== undefined
          ? { priority: this.options.defaultPriority }
          : {}),
        ...(this.options.defaultCategory
          ? { category: this.options.defaultCategory }
          : {}),
        metadata: {
          ...this.options.defaultMetadata,
          ...this.commonOptions.metadata,
        },
      };

      const createdRule = await this.rulesEngine.create(ruleInput);
      createdRules.push(createdRule);
    }

    return createdRules;
  }

  async test(context: RuleContext): Promise<RuleEvaluationResult[]> {
    const results: RuleEvaluationResult[] = [];

    for (const rule of this.rules) {
      const ruleInput: RuleCreateInput = {
        ...rule,
        ...this.commonOptions,
        enabled: true,
      };

      const builderResult = new RuleBuilderResult(ruleInput, this.rulesEngine);
      const testResult = await builderResult.test(context);
      results.push(testResult);
    }

    return results;
  }
}

export class ConditionalGroupBuilder {
  private group: Partial<ConditionalRuleGroup>;

  constructor(
    name: string,
    private builder: RuleBuilder,
  ) {
    this.group = { name, rules: [] };
  }

  addRule(rule: Rule): this {
    this.group.rules = this.group.rules || [];
    this.group.rules.push(rule);
    return this;
  }

  addRules(rules: Rule[]): this {
    this.group.rules = this.group.rules || [];
    this.group.rules.push(...rules);
    return this;
  }

  enableWhen(
    condition: (context: RuleContext) => boolean | Promise<boolean>,
  ): this {
    this.group.enableCondition = condition;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.group.metadata = metadata;
    return this;
  }

  build(): ConditionalRuleGroup {
    if (!this.group.rules || this.group.rules.length === 0) {
      throw new RuleValidationError(
        ["At least one rule is required for conditional groups"],
        { group: this.group },
      );
    }

    return this.group as ConditionalRuleGroup;
  }

  register(): this {
    this.builder.registerConditionalGroup(this.build());
    return this;
  }
}

export class RuleChainBuilder {
  private chain: Partial<RuleChain>;

  constructor(
    name: string,
    private builder: RuleBuilder,
  ) {
    this.chain = { name, rules: [] };
  }

  addRule(rule: Rule): this {
    this.chain.rules = this.chain.rules || [];
    this.chain.rules.push(rule);
    return this;
  }

  addRules(rules: Rule[]): this {
    this.chain.rules = this.chain.rules || [];
    this.chain.rules.push(...rules);
    return this;
  }

  stopOnFailure(stop: boolean = true): this {
    this.chain.stopOnFailure = stop;
    return this;
  }

  transformContext(
    transformer: (context: RuleContext, ruleIndex: number) => RuleContext,
  ): this {
    this.chain.contextTransformer = transformer;
    return this;
  }

  build(): RuleChain {
    if (!this.chain.rules || this.chain.rules.length === 0) {
      throw new RuleValidationError(
        ["At least one rule is required for rule chains"],
        { chain: this.chain },
      );
    }

    return this.chain as RuleChain;
  }

  register(): this {
    this.builder.registerChain(this.build());
    return this;
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new RuleBuilder instance
 */
export function createRuleBuilder(
  rulesEngine: RulesEngine,
  options?: RuleBuilderOptions,
): RuleBuilder {
  return new RuleBuilder(rulesEngine, options);
}

/**
 * Create a rule template
 */
export function createRuleTemplate(
  name: string,
  type: RuleType,
  options: Partial<RuleTemplate> = {},
): RuleTemplate {
  return {
    name,
    type,
    ...options,
  };
}

/**
 * Create common rule templates
 */
export const CommonTemplates = {
  /**
   * Template for resource state checking
   */
  resourceState: (stateName: string): RuleTemplate => ({
    name: "resource-state-check",
    type: "simple",
    condition: "resource.state == '{{state}}'",
    parameters: ["state"],
    category: "resource",
    description: `Check if resource is in ${stateName} state`,
  }),

  /**
   * Template for resource data field validation
   */
  resourceDataField: (fieldName: string): RuleTemplate => ({
    name: "resource-data-field",
    type: "simple",
    condition: "resource.data.{{field}} == '{{value}}'",
    parameters: ["field", "value"],
    category: "validation",
    description: `Check resource data field ${fieldName}`,
  }),

  /**
   * Template for time-based rules
   */
  timeWindow: (): RuleTemplate => ({
    name: "time-window",
    type: "javascript",
    condition:
      "context.timestamp >= new Date('{{startTime}}') && context.timestamp <= new Date('{{endTime}}')",
    parameters: ["startTime", "endTime"],
    category: "temporal",
    description: "Check if current time is within specified window",
  }),

  /**
   * Template for metadata validation
   */
  metadataCheck: (): RuleTemplate => ({
    name: "metadata-check",
    type: "simple",
    condition: "resource.metadata.{{key}} == '{{value}}'",
    parameters: ["key", "value"],
    category: "metadata",
    description: "Check resource metadata value",
  }),
};
