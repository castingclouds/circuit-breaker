/**
 * Rules Engine for Circuit Breaker TypeScript SDK
 *
 * This file provides comprehensive rule management functionality including
 * rule evaluation, validation, CRUD operations, and support for various
 * rule types (simple, composite, custom, javascript).
 */

import {
  Rule,
  RuleType,
  CompositeRule,
  RuleContext,
  RuleEvaluationResult,
  RuleResult,
  RuleValidationResult,
  RuleEvaluator,
  RulesConfig,
  PaginationOptions,
  PaginatedResult,
} from "../core/types.js";
import {
  RuleEvaluationError,
  RuleNotFoundError,
  RuleValidationError,
  RuleTimeoutError,
  ErrorHandler,
} from "../core/errors.js";
import { GraphQLClient } from "../utils/graphql.js";
import { Logger, sanitizeForLogging } from "../utils/logger.js";

// ============================================================================
// Logger Context Type
// ============================================================================

interface LogContext {
  operation?: string;
  ruleId?: string;
  ruleName?: string;
  ruleType?: string;
  requestId?: string;
  component?: string;
  userId?: string;
  correlationId?: string;
}

// ============================================================================
// Extended Types
// ============================================================================

export interface RuleCreateInput {
  name: string;
  type: RuleType;
  condition?: string;
  evaluator?: RuleEvaluator;
  description?: string;
  category?: string;
  metadata?: Record<string, any>;
  priority?: number;
  enabled?: boolean;
  // For composite rules
  operator?: "AND" | "OR" | "NOT";
  rules?: Rule[];
}

export interface RuleUpdateInput {
  name?: string;
  condition?: string;
  description?: string;
  category?: string;
  metadata?: Record<string, any>;
  priority?: number;
  enabled?: boolean;
  // For composite rules
  operator?: "AND" | "OR" | "NOT";
  rules?: Rule[];
}

export interface RuleSearchOptions extends PaginationOptions {
  /** Search in rule names and descriptions */
  query?: string;

  /** Filter by rule type */
  type?: RuleType;
  types?: RuleType[];

  /** Filter by category */
  category?: string;
  categories?: string[];

  /** Filter by enabled status */
  enabled?: boolean;

  /** Filter by priority range */
  minPriority?: number;
  maxPriority?: number;

  /** Include rule evaluation statistics */
  includeStats?: boolean;

  /** Sort field */
  sortBy?: string;

  /** Sort direction */
  sortDirection?: "asc" | "desc";
}

export interface RuleStats {
  /** Total number of rules */
  totalRules: number;

  /** Rules by type */
  byType: Record<RuleType, number>;

  /** Rules by category */
  byCategory: Record<string, number>;

  /** Enabled vs disabled rules */
  enabled: number;
  disabled: number;

  /** Average evaluation time */
  averageEvaluationTime: number;

  /** Most frequently evaluated rules */
  mostEvaluated: { rule: Rule; evaluations: number }[];

  /** Recent evaluation activity */
  recentActivity: number;
}

export interface RuleWithStats extends Rule {
  id: string;
  createdAt: string;
  updatedAt: string;
  stats?: {
    evaluations: number;
    successRate: number;
    averageExecutionTime: number;
    lastEvaluation?: Date;
  };
}

export interface RuleEvaluationOptions {
  /** Maximum evaluation timeout */
  timeout?: number;

  /** Whether to stop on first failure */
  stopOnFailure?: boolean;

  /** Whether to include detailed execution info */
  includeDetails?: boolean;

  /** Custom evaluator overrides */
  customEvaluators?: Record<string, RuleEvaluator>;

  /** Whether to use cached results */
  useCache?: boolean;
}

export interface BatchRuleEvaluationInput {
  /** Rules to evaluate (can be rule names or Rule objects) */
  rules: (string | Rule)[];

  /** Evaluation context */
  context: RuleContext;

  /** Evaluation options */
  options?: RuleEvaluationOptions;
}

export interface BatchRuleEvaluationResult {
  /** Overall success status */
  success: boolean;

  /** Individual rule results */
  results: RuleEvaluationResult[];

  /** Number of successful evaluations */
  successful: number;

  /** Number of failed evaluations */
  failed: number;

  /** Total evaluation time */
  totalTime: number;
}

export interface RuleHealthStatus {
  healthy: boolean;
  issues: string[];
  lastCheck: Date;
  failingRules: number;
  errorRate: number;
  avgEvaluationTime: number;
  cachingEfficiency?: number;
}

// ============================================================================
// Rules Engine
// ============================================================================

export class RulesEngine {
  private readonly graphqlClient: GraphQLClient;
  private readonly logger: Logger;
  private readonly config: RulesConfig;
  private readonly cache = new Map<string, RuleEvaluationResult>();
  private readonly ruleCache = new Map<string, Rule>();
  private readonly cacheTimeout = 5 * 60 * 1000; // 5 minutes
  private readonly maxCacheSize = 1000;
  private readonly customEvaluators = new Map<string, RuleEvaluator>();

  constructor(
    graphqlClient: GraphQLClient,
    logger: Logger,
    config: RulesConfig = {},
  ) {
    this.graphqlClient = graphqlClient;
    this.logger = logger;
    this.config = {
      enableCache: true,
      cacheSize: 1000,
      evaluationTimeout: 30000,
      strictMode: false,
      ...config,
    };

    // Register custom rules from config
    if (config.customRules) {
      Object.entries(config.customRules).forEach(([name, rule]) => {
        if (rule.evaluator) {
          this.customEvaluators.set(name, rule.evaluator);
        }
      });
    }
  }

  // ============================================================================
  // Core CRUD Operations
  // ============================================================================

  /**
   * Create a new rule
   */
  async create(
    input: RuleCreateInput,
    options: { validate?: boolean } = {},
  ): Promise<Rule> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_create",
      ruleName: input.name,
      ruleType: input.type,
      requestId,
    };

    this.logger.info("Creating new rule", context);

    try {
      // Validate input
      if (options.validate !== false) {
        await this.validateRuleInput(input);
      }

      // Create rule via GraphQL
      const mutation = `
        mutation CreateRule($input: RuleCreateInput!) {
          createRule(input: $input) {
            id
            name
            type
            condition
            description
            category
            metadata
            priority
            enabled
            operator
            rules {
              id
              name
              type
              condition
            }
            createdAt
            updatedAt
          }
        }
      `;

      const variables = {
        input: {
          name: input.name,
          type: input.type,
          condition: input.condition,
          description: input.description,
          category: input.category,
          metadata: input.metadata || {},
          priority: input.priority || 0,
          enabled: input.enabled !== false,
          ...(input.type === "composite"
            ? {
                operator: input.operator || "AND",
                rules: input.rules || [],
              }
            : {}),
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const rule = response.createRule;

      // Cache the rule
      this.setRuleCache(rule.name, rule);

      // Register custom evaluator if provided
      if (input.evaluator) {
        this.customEvaluators.set(rule.name, input.evaluator);
      }

      this.logger.info("Rule created successfully", {
        ...context,
        ruleId: rule.id,
        enabled: rule.enabled,
      });

      return rule;
    } catch (error) {
      this.logger.error("Failed to create rule", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get a rule by name
   */
  async get(
    ruleName: string,
    options: { useCache?: boolean } = {},
  ): Promise<Rule> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_get",
      ruleName,
      requestId,
    };

    this.logger.debug("Getting rule", context);

    try {
      // Check cache first
      if (options.useCache !== false) {
        const cached = this.getRuleCache(ruleName);
        if (cached) {
          this.logger.debug("Rule found in cache", context);
          return cached;
        }
      }

      // Fetch from API
      const query = `
        query GetRule($name: String!) {
          rule(name: $name) {
            id
            name
            type
            condition
            description
            category
            metadata
            priority
            enabled
            operator
            rules {
              id
              name
              type
              condition
              description
              enabled
            }
            createdAt
            updatedAt
          }
        }
      `;

      const variables = { name: ruleName };
      const response = await this.graphqlClient.request(query, variables);

      if (!response.rule) {
        throw new RuleNotFoundError(ruleName, requestId);
      }

      const rule = response.rule;

      // Cache the rule
      this.setRuleCache(ruleName, rule);

      this.logger.debug("Rule retrieved successfully", {
        ...context,
        ruleId: rule.id,
        ruleType: rule.type,
      });

      return rule;
    } catch (error) {
      this.logger.error("Failed to get rule", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Update a rule
   */
  async update(
    ruleName: string,
    input: RuleUpdateInput,
    options: { validate?: boolean } = {},
  ): Promise<Rule> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_update",
      ruleName,
      requestId,
    };

    this.logger.info("Updating rule", context);

    try {
      // Get existing rule
      const existingRule = await this.get(ruleName, { useCache: false });

      if (options.validate !== false) {
        await this.validateRuleUpdate(existingRule, input);
      }

      // Update via GraphQL
      const mutation = `
        mutation UpdateRule($name: String!, $input: RuleUpdateInput!) {
          updateRule(name: $name, input: $input) {
            id
            name
            type
            condition
            description
            category
            metadata
            priority
            enabled
            operator
            rules {
              id
              name
              type
              condition
            }
            createdAt
            updatedAt
          }
        }
      `;

      const variables = {
        name: ruleName,
        input: {
          name: input.name,
          condition: input.condition,
          description: input.description,
          category: input.category,
          metadata: input.metadata,
          priority: input.priority,
          enabled: input.enabled,
          ...(input.operator ? { operator: input.operator } : {}),
          ...(input.rules ? { rules: input.rules } : {}),
        },
      };

      const response = await this.graphqlClient.request(mutation, variables);
      const rule = response.updateRule;

      // Update cache
      this.setRuleCache(ruleName, rule);
      if (input.name && input.name !== ruleName) {
        this.ruleCache.delete(ruleName);
        this.setRuleCache(input.name, rule);
      }

      this.logger.info("Rule updated successfully", {
        ...context,
        ruleId: rule.id,
        enabled: rule.enabled,
      });

      return rule;
    } catch (error) {
      this.logger.error("Failed to update rule", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Delete a rule
   */
  async delete(
    ruleName: string,
    options: { force?: boolean } = {},
  ): Promise<boolean> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_delete",
      ruleName,
      requestId,
    };

    this.logger.info("Deleting rule", context);

    try {
      // Check if rule is referenced by other rules (unless force delete)
      if (!options.force) {
        const dependencies = await this.findRuleDependencies(ruleName);
        if (dependencies.length > 0) {
          throw new RuleValidationError(
            [
              `Cannot delete rule '${ruleName}' as it is referenced by: ${dependencies.join(", ")}`,
            ],
            { ruleName, dependencies },
            requestId,
          );
        }
      }

      // Delete via GraphQL
      const mutation = `
        mutation DeleteRule($name: String!, $force: Boolean) {
          deleteRule(name: $name, force: $force) {
            success
          }
        }
      `;

      const variables = { name: ruleName, force: options.force };
      const response = await this.graphqlClient.request(mutation, variables);

      if (response.deleteRule.success) {
        // Remove from caches
        this.ruleCache.delete(ruleName);
        this.customEvaluators.delete(ruleName);
        this.clearEvaluationCache(ruleName);

        this.logger.info("Rule deleted successfully", context);
        return true;
      }

      return false;
    } catch (error) {
      this.logger.error("Failed to delete rule", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Search and List Operations
  // ============================================================================

  /**
   * List rules with pagination and filtering
   */
  async list(
    options: RuleSearchOptions = {},
  ): Promise<PaginatedResult<RuleWithStats>> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_list",
      requestId,
    };

    this.logger.debug("Listing rules", {
      ...context,
      options: sanitizeForLogging(options),
    });

    try {
      const args: any = {
        limit: options.limit || 50,
        offset: options.offset || 0,
      };

      // Add filters
      const filters = this.buildFilters(options);
      if (Object.keys(filters).length > 0) {
        args.filters = filters;
      }

      // Add sorting
      if (options.sortBy) {
        args.sortBy = options.sortBy;
        args.sortDirection = options.sortDirection || "asc";
      }

      const statsFields = options.includeStats
        ? `stats { evaluations, successRate, averageExecutionTime, lastEvaluation }`
        : "";

      const query = `
        query ListRules($limit: Int, $offset: Int, $filters: RuleFilters, $sortBy: String, $sortDirection: String) {
          rules(limit: $limit, offset: $offset, filters: $filters, sortBy: $sortBy, sortDirection: $sortDirection) {
            items {
              id
              name
              type
              condition
              description
              category
              metadata
              priority
              enabled
              operator
              rules {
                name
                type
                enabled
              }
              createdAt
              updatedAt
              ${statsFields}
            }
            totalCount
            hasMore
          }
        }
      `;

      const response = await this.graphqlClient.request(query, args);
      const result = response.rules;

      this.logger.debug("Rules listed successfully", {
        ...context,
        count: result.items.length,
        totalCount: result.totalCount,
      });

      return {
        data: result.items,
        totalCount: result.totalCount,
        hasMore: result.hasMore,
        limit: options.limit || 50,
        offset: options.offset || 0,
      };
    } catch (error) {
      this.logger.error("Failed to list rules", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Search rules (alias for list with search-specific defaults)
   */
  async search(
    options: RuleSearchOptions,
  ): Promise<PaginatedResult<RuleWithStats>> {
    return this.list({
      ...options,
      includeStats: options.includeStats !== false,
    });
  }

  // ============================================================================
  // Rule Evaluation
  // ============================================================================

  /**
   * Evaluate a single rule
   */
  async evaluateRule(
    ruleName: string,
    context: RuleContext,
    options: RuleEvaluationOptions = {},
  ): Promise<RuleEvaluationResult> {
    const requestId = this.generateRequestId();
    const logContext: LogContext = {
      operation: "rule_evaluate",
      ruleName,
      requestId,
    };

    this.logger.debug("Evaluating rule", logContext);

    const startTime = Date.now();

    try {
      // Check cache first
      if (options.useCache !== false && this.config.enableCache) {
        const cacheKey = this.generateCacheKey(ruleName, context);
        const cached = this.getEvaluationCache(cacheKey);
        if (cached) {
          this.logger.debug("Rule evaluation found in cache", logContext);
          return cached;
        }
      }

      // Get the rule
      const rule = await this.get(ruleName);

      if (!rule.enabled) {
        this.logger.debug("Rule is disabled, skipping evaluation", logContext);
        return {
          passed: true, // Disabled rules pass by default
          results: [
            {
              rule,
              passed: true,
              context: { disabled: true },
              executionTime: 0,
            },
          ],
          errors: [],
          evaluationTime: Date.now() - startTime,
          rulesEvaluated: 0,
          rulesPassed: 0,
        };
      }

      // Evaluate the rule
      const ruleResult = await this.evaluateSingleRule(rule, context, options);

      const result: RuleEvaluationResult = {
        passed: ruleResult.passed,
        results: [ruleResult],
        errors: ruleResult.error ? [ruleResult.error] : [],
        evaluationTime: Date.now() - startTime,
        rulesEvaluated: 1,
        rulesPassed: ruleResult.passed ? 1 : 0,
      };

      // Cache the result
      if (this.config.enableCache) {
        const cacheKey = this.generateCacheKey(ruleName, context);
        this.setEvaluationCache(cacheKey, result);
      }

      this.logger.debug("Rule evaluation completed", {
        ...logContext,
        passed: result.passed,
        evaluationTime: result.evaluationTime,
      });

      return result;
    } catch (error) {
      this.logger.error("Failed to evaluate rule", logContext, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Evaluate multiple rules
   */
  async evaluateRules(
    ruleNames: string[],
    context: RuleContext,
    options: RuleEvaluationOptions = {},
  ): Promise<RuleEvaluationResult> {
    const requestId = this.generateRequestId();
    const logContext: LogContext = {
      operation: "rules_evaluate",
      requestId,
    };

    this.logger.debug("Evaluating multiple rules", {
      ...logContext,
      ruleCount: ruleNames.length,
    });

    const startTime = Date.now();
    const results: RuleResult[] = [];
    const errors: string[] = [];
    let rulesPassed = 0;

    try {
      for (const ruleName of ruleNames) {
        try {
          const ruleEvaluation = await this.evaluateRule(
            ruleName,
            context,
            options,
          );
          results.push(...ruleEvaluation.results);
          errors.push(...ruleEvaluation.errors);
          if (ruleEvaluation.passed) {
            rulesPassed++;
          }

          // Stop on first failure if configured
          if (options.stopOnFailure && !ruleEvaluation.passed) {
            break;
          }
        } catch (error) {
          const errorMsg = `Failed to evaluate rule '${ruleName}': ${error}`;
          errors.push(errorMsg);
          this.logger.warn("Rule evaluation failed", { ruleName, error });

          if (options.stopOnFailure) {
            break;
          }
        }
      }

      const result: RuleEvaluationResult = {
        passed: errors.length === 0 && rulesPassed === results.length,
        results,
        errors,
        evaluationTime: Date.now() - startTime,
        rulesEvaluated: results.length,
        rulesPassed,
      };

      this.logger.debug("Multiple rules evaluation completed", {
        ...logContext,
        passed: result.passed,
        rulesEvaluated: result.rulesEvaluated,
        rulesPassed: result.rulesPassed,
        evaluationTime: result.evaluationTime,
      });

      return result;
    } catch (error) {
      this.logger.error("Failed to evaluate multiple rules", logContext, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Batch rule evaluation
   */
  async evaluateBatch(
    inputs: BatchRuleEvaluationInput[],
  ): Promise<BatchRuleEvaluationResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rules_evaluate_batch",
      requestId,
    };

    this.logger.info("Evaluating rules in batch", {
      ...context,
      batchCount: inputs.length,
    });

    const startTime = Date.now();
    const results: RuleEvaluationResult[] = [];
    let successful = 0;
    let failed = 0;

    try {
      for (const input of inputs) {
        try {
          const ruleNames = input.rules.map((r) =>
            typeof r === "string" ? r : r.name,
          );

          const result = await this.evaluateRules(
            ruleNames,
            input.context,
            input.options,
          );

          results.push(result);
          if (result.passed) {
            successful++;
          } else {
            failed++;
          }
        } catch (error) {
          failed++;
          this.logger.warn("Batch rule evaluation item failed", { error });
        }
      }

      const batchResult: BatchRuleEvaluationResult = {
        success: failed === 0,
        results,
        successful,
        failed,
        totalTime: Date.now() - startTime,
      };

      this.logger.info("Batch rule evaluation completed", {
        ...context,
        successful,
        failed,
        totalTime: batchResult.totalTime,
      });

      return batchResult;
    } catch (error) {
      this.logger.error("Failed to evaluate rules in batch", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Rule Validation
  // ============================================================================

  /**
   * Validate a rule definition
   */
  async validateRule(
    rule: Rule | RuleCreateInput,
    options: { deep?: boolean } = {},
  ): Promise<RuleValidationResult> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_validate",
      ruleName: rule.name,
      ruleType: rule.type,
      requestId,
    };

    this.logger.debug("Validating rule", context);

    try {
      const errors: string[] = [];
      const warnings: string[] = [];

      // Basic validation
      if (!rule.name || rule.name.trim() === "") {
        errors.push("Rule name is required");
      }

      if (!rule.type) {
        errors.push("Rule type is required");
      } else if (
        !["simple", "composite", "custom", "javascript"].includes(rule.type)
      ) {
        errors.push(`Invalid rule type: ${rule.type}`);
      }

      // Type-specific validation
      switch (rule.type) {
        case "simple":
        case "javascript":
          if (!rule.condition) {
            errors.push("Condition is required for simple/javascript rules");
          } else {
            // Validate condition syntax for javascript rules
            if (rule.type === "javascript") {
              try {
                new Function("context", rule.condition);
              } catch (error) {
                errors.push(`Invalid JavaScript condition: ${error}`);
              }
            }
          }
          break;

        case "composite":
          const compositeRule = rule as CompositeRule;
          if (!compositeRule.operator) {
            errors.push("Operator is required for composite rules");
          } else if (!["AND", "OR", "NOT"].includes(compositeRule.operator)) {
            errors.push(`Invalid operator: ${compositeRule.operator}`);
          }

          if (!compositeRule.rules || compositeRule.rules.length === 0) {
            errors.push("Child rules are required for composite rules");
          } else {
            // Recursively validate child rules if deep validation is enabled
            if (options.deep) {
              for (const childRule of compositeRule.rules) {
                const childValidation = await this.validateRule(
                  childRule,
                  options,
                );
                errors.push(
                  ...childValidation.errors.map(
                    (e) => `Child rule '${childRule.name}': ${e}`,
                  ),
                );
                warnings.push(
                  ...childValidation.warnings.map(
                    (w) => `Child rule '${childRule.name}': ${w}`,
                  ),
                );
              }
            }
          }
          break;

        case "custom":
          if (!rule.evaluator && !this.customEvaluators.has(rule.name)) {
            errors.push("Custom evaluator is required for custom rules");
          }
          break;
      }

      // Priority validation
      if (
        rule.priority !== undefined &&
        (rule.priority < 0 || rule.priority > 1000)
      ) {
        warnings.push("Priority should be between 0 and 1000");
      }

      const result: RuleValidationResult = {
        valid: errors.length === 0,
        errors,
        warnings,
        ruleCount: 1,
      };

      this.logger.debug("Rule validation completed", {
        ...context,
        valid: result.valid,
        errors: result.errors.length,
        warnings: result.warnings.length,
      });

      return result;
    } catch (error) {
      this.logger.error("Failed to validate rule", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Analytics and Monitoring
  // ============================================================================

  /**
   * Get rule statistics
   */
  async getStats(): Promise<RuleStats> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_stats",
      requestId,
    };

    this.logger.debug("Getting rule statistics", context);

    try {
      const query = `
        query RuleStats {
          ruleStats {
            totalRules
            byType
            byCategory
            enabled
            disabled
            averageEvaluationTime
            mostEvaluated {
              rule {
                name
                type
                category
              }
              evaluations
            }
            recentActivity
          }
        }
      `;

      const response = await this.graphqlClient.request(query);
      const stats = response.ruleStats;

      this.logger.debug("Rule statistics retrieved", {
        ...context,
        totalRules: stats.totalRules,
        enabled: stats.enabled,
      });

      return stats;
    } catch (error) {
      this.logger.error("Failed to get rule statistics", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  /**
   * Get rule health status
   */
  async getHealth(): Promise<RuleHealthStatus> {
    const requestId = this.generateRequestId();
    const context: LogContext = {
      operation: "rule_health",
      requestId,
    };

    this.logger.debug("Getting rule health status", context);

    try {
      const query = `
        query RuleHealth {
          ruleHealth {
            healthy
            issues
            lastCheck
            failingRules
            errorRate
            avgEvaluationTime
            cachingEfficiency
          }
        }
      `;

      const response = await this.graphqlClient.request(query);
      const health = response.ruleHealth;

      this.logger.debug("Rule health status retrieved", {
        ...context,
        healthy: health.healthy,
        issues: health.issues.length,
      });

      return {
        ...health,
        lastCheck: new Date(health.lastCheck),
      };
    } catch (error) {
      this.logger.error("Failed to get rule health", context, error);
      throw ErrorHandler.handle(error, requestId);
    }
  }

  // ============================================================================
  // Custom Evaluators
  // ============================================================================

  /**
   * Register a custom evaluator
   */
  registerEvaluator(name: string, evaluator: RuleEvaluator): void {
    this.customEvaluators.set(name, evaluator);
    this.logger.debug("Custom evaluator registered", { name });
  }

  /**
   * Unregister a custom evaluator
   */
  unregisterEvaluator(name: string): boolean {
    const removed = this.customEvaluators.delete(name);
    if (removed) {
      this.logger.debug("Custom evaluator unregistered", { name });
    }
    return removed;
  }

  /**
   * Get registered evaluators
   */
  getRegisteredEvaluators(): string[] {
    return Array.from(this.customEvaluators.keys());
  }

  // ============================================================================
  // Manager Lifecycle
  // ============================================================================

  /**
   * Initialize the rules engine
   */
  async initialize(): Promise<void> {
    const context: LogContext = { operation: "rules_engine_init" };
    this.logger.info("Initializing RulesEngine", context);

    try {
      // Perform any initialization tasks
      // This could include warming up caches, validating connections, etc.

      this.logger.info("RulesEngine initialized successfully", context);
    } catch (error) {
      this.logger.error("Failed to initialize RulesEngine", context, error);
      throw error;
    }
  }

  /**
   * Get engine health status
   */
  async getEngineHealth(): Promise<{
    healthy: boolean;
    cacheSize: number;
    customEvaluators: number;
    lastActivity?: Date;
  }> {
    return {
      healthy: true,
      cacheSize: this.cache.size,
      customEvaluators: this.customEvaluators.size,
      lastActivity: new Date(),
    };
  }

  /**
   * Reset the engine (clear caches, etc.)
   */
  async reset(): Promise<void> {
    this.cache.clear();
    this.ruleCache.clear();
  }

  /**
   * Dispose of the engine and clean up resources
   */
  async dispose(): Promise<void> {
    this.cache.clear();
    this.ruleCache.clear();
    this.customEvaluators.clear();
  }

  // ============================================================================
  // Private Helper Methods
  // ============================================================================

  /**
   * Evaluate a single rule
   */
  private async evaluateSingleRule(
    rule: Rule,
    context: RuleContext,
    options: RuleEvaluationOptions = {},
  ): Promise<RuleResult> {
    const startTime = Date.now();

    try {
      let passed = false;

      switch (rule.type) {
        case "simple":
          passed = await this.evaluateSimpleRule(rule, context);
          break;

        case "javascript":
          passed = await this.evaluateJavaScriptRule(rule, context);
          break;

        case "composite":
          passed = await this.evaluateCompositeRule(
            rule as CompositeRule,
            context,
            options,
          );
          break;

        case "custom":
          passed = await this.evaluateCustomRule(rule, context, options);
          break;

        default:
          throw new RuleEvaluationError(rule.name, [
            `Unsupported rule type: ${rule.type}`,
          ]);
      }

      return {
        rule,
        passed,
        executionTime: Date.now() - startTime,
      };
    } catch (error) {
      return {
        rule,
        passed: false,
        error: error instanceof Error ? error.message : String(error),
        executionTime: Date.now() - startTime,
      };
    }
  }

  /**
   * Evaluate a simple rule
   */
  private async evaluateSimpleRule(
    rule: Rule,
    context: RuleContext,
  ): Promise<boolean> {
    if (!rule.condition) {
      throw new Error("Simple rule must have a condition");
    }

    // Simple string-based condition evaluation
    // This is a basic implementation - in production, you'd want a more sophisticated parser
    const condition = rule.condition.toLowerCase();

    // Basic pattern matching for common conditions
    if (condition.includes("resource.state")) {
      const targetState = condition.match(/==\s*["']([^"']+)["']/)?.[1];
      if (targetState) {
        return context.resource.state === targetState;
      }
    }

    if (condition.includes("resource.data")) {
      // Basic data field checking
      const fieldMatch = condition.match(/resource\.data\.(\w+)/);
      const valueMatch = condition.match(/==\s*(.+)/);

      if (fieldMatch && valueMatch) {
        const field = fieldMatch[1];
        const expectedValue = valueMatch[1].replace(/["']/g, "");
        return context.resource.data[field] === expectedValue;
      }
    }

    // Default to true for simple conditions we can't parse
    return true;
  }

  /**
   * Evaluate a JavaScript rule
   */
  private async evaluateJavaScriptRule(
    rule: Rule,
    context: RuleContext,
  ): Promise<boolean> {
    if (!rule.condition) {
      throw new Error("JavaScript rule must have a condition");
    }

    try {
      // Create a safe evaluation context
      const evaluationContext = {
        resource: context.resource,
        workflow: context.workflow,
        activity: context.activity,
        metadata: context.metadata,
        timestamp: context.timestamp,
      };

      // Create and execute the function
      const func = new Function("context", `return ${rule.condition}`);
      const result = func(evaluationContext);

      // Handle both sync and async results
      return await Promise.resolve(result);
    } catch (error) {
      throw new RuleEvaluationError(rule.name, [
        `JavaScript evaluation failed: ${error}`,
      ]);
    }
  }

  /**
   * Evaluate a composite rule
   */
  private async evaluateCompositeRule(
    rule: CompositeRule,
    context: RuleContext,
    options: RuleEvaluationOptions,
  ): Promise<boolean> {
    if (!rule.rules || rule.rules.length === 0) {
      throw new Error("Composite rule must have child rules");
    }

    const results: boolean[] = [];

    for (const childRule of rule.rules) {
      const childResult = await this.evaluateSingleRule(
        childRule,
        context,
        options,
      );
      results.push(childResult.passed);

      // Short-circuit evaluation for performance
      if (rule.operator === "AND" && !childResult.passed) {
        return false;
      }
      if (rule.operator === "OR" && childResult.passed) {
        return true;
      }
    }

    switch (rule.operator) {
      case "AND":
        return results.every((r) => r);
      case "OR":
        return results.some((r) => r);
      case "NOT":
        return !results[0]; // NOT operator applies to first rule only
      default:
        throw new Error(`Unknown operator: ${rule.operator}`);
    }
  }

  /**
   * Evaluate a custom rule
   */
  private async evaluateCustomRule(
    rule: Rule,
    context: RuleContext,
    options: RuleEvaluationOptions,
  ): Promise<boolean> {
    // Check for custom evaluator override
    const customEvaluator =
      options.customEvaluators?.[rule.name] ||
      rule.evaluator ||
      this.customEvaluators.get(rule.name);

    if (!customEvaluator) {
      throw new RuleEvaluationError(rule.name, [
        "No custom evaluator found for custom rule",
      ]);
    }

    try {
      // Set timeout for custom evaluator
      const timeout = options.timeout || this.config.evaluationTimeout || 30000;

      return await Promise.race([
        Promise.resolve(customEvaluator(context)),
        new Promise<never>((_, reject) => {
          setTimeout(
            () => reject(new RuleTimeoutError(rule.name, timeout)),
            timeout,
          );
        }),
      ]);
    } catch (error) {
      if (error instanceof RuleTimeoutError) {
        throw error;
      }
      throw new RuleEvaluationError(rule.name, [
        `Custom evaluator failed: ${error}`,
      ]);
    }
  }

  /**
   * Validate rule input
   */
  private async validateRuleInput(input: RuleCreateInput): Promise<void> {
    if (!input.name) {
      throw new RuleValidationError(["Rule name is required"], { input });
    }

    if (!input.type) {
      throw new RuleValidationError(["Rule type is required"], { input });
    }

    // Check if rule with same name already exists
    try {
      await this.get(input.name, { useCache: false });
      throw new RuleValidationError(
        [`Rule with name '${input.name}' already exists`],
        { input },
      );
    } catch (error) {
      if (!(error instanceof RuleNotFoundError)) {
        throw error;
      }
      // Rule doesn't exist, which is what we want
    }
  }

  /**
   * Validate rule update
   */
  private async validateRuleUpdate(
    existingRule: Rule,
    input: RuleUpdateInput,
  ): Promise<void> {
    // Basic validation - can be extended
    if (input.name && input.name !== existingRule.name) {
      // Check if new name already exists
      try {
        await this.get(input.name, { useCache: false });
        throw new RuleValidationError(
          [`Rule with name '${input.name}' already exists`],
          { input },
        );
      } catch (error) {
        if (!(error instanceof RuleNotFoundError)) {
          throw error;
        }
      }
    }
  }

  /**
   * Find rules that depend on a given rule
   */
  private async findRuleDependencies(ruleName: string): Promise<string[]> {
    try {
      const query = `
        query FindRuleDependencies($ruleName: String!) {
          ruleDependencies(ruleName: $ruleName) {
            dependentRules
          }
        }
      `;

      const response = await this.graphqlClient.request(query, { ruleName });
      return response.ruleDependencies?.dependentRules || [];
    } catch (error) {
      // If endpoint doesn't exist, return empty array
      this.logger.warn("Could not check rule dependencies", {
        ruleName,
        error,
      });
      return [];
    }
  }

  /**
   * Build filters for rule queries
   */
  private buildFilters(options: RuleSearchOptions): Record<string, any> {
    const filters: Record<string, any> = {};

    if (options.query) {
      filters.search = options.query;
    }

    if (options.type) {
      filters.type = options.type;
    }

    if (options.types && options.types.length > 0) {
      filters.types = options.types;
    }

    if (options.category) {
      filters.category = options.category;
    }

    if (options.categories && options.categories.length > 0) {
      filters.categories = options.categories;
    }

    if (options.enabled !== undefined) {
      filters.enabled = options.enabled;
    }

    if (options.minPriority !== undefined) {
      filters.minPriority = options.minPriority;
    }

    if (options.maxPriority !== undefined) {
      filters.maxPriority = options.maxPriority;
    }

    return filters;
  }

  /**
   * Get rule from cache
   */
  private getRuleCache(ruleName: string): Rule | undefined {
    return this.ruleCache.get(ruleName);
  }

  /**
   * Set rule in cache
   */
  private setRuleCache(ruleName: string, rule: Rule): void {
    this.ruleCache.set(ruleName, rule);

    // Clean cache if it gets too large
    if (this.ruleCache.size > this.maxCacheSize) {
      const firstKey = this.ruleCache.keys().next().value;
      if (firstKey) {
        this.ruleCache.delete(firstKey);
      }
    }
  }

  /**
   * Generate cache key for evaluation results
   */
  private generateCacheKey(ruleName: string, context: RuleContext): string {
    const contextHash = JSON.stringify({
      resourceId: context.resource.id,
      resourceState: context.resource.state,
      activityId: context.activity.id,
      timestamp: Math.floor(context.timestamp.getTime() / 60000), // Group by minute
    });
    return `${ruleName}:${btoa(contextHash)}`;
  }

  /**
   * Get evaluation from cache
   */
  private getEvaluationCache(
    cacheKey: string,
  ): RuleEvaluationResult | undefined {
    return this.cache.get(cacheKey);
  }

  /**
   * Set evaluation in cache
   */
  private setEvaluationCache(
    cacheKey: string,
    result: RuleEvaluationResult,
  ): void {
    this.cache.set(cacheKey, result);

    // Clean cache if it gets too large
    if (this.cache.size > this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }

    // Set timeout to clear cache entry
    setTimeout(() => {
      this.cache.delete(cacheKey);
    }, this.cacheTimeout);
  }

  /**
   * Clear evaluation cache for a specific rule
   */
  private clearEvaluationCache(ruleName: string): void {
    const keysToDelete: string[] = [];
    for (const key of this.cache.keys()) {
      if (key.startsWith(`${ruleName}:`)) {
        keysToDelete.push(key);
      }
    }
    keysToDelete.forEach((key) => this.cache.delete(key));
  }

  /**
   * Generate a unique request ID for tracking
   */
  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new RulesEngine instance
 */
export function createRulesEngine(
  graphqlClient: GraphQLClient,
  logger: Logger,
  config?: RulesConfig,
): RulesEngine {
  return new RulesEngine(graphqlClient, logger, config);
}

/**
 * Validate a rule definition
 */
export function validateRuleDefinition(rule: Rule): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  // Basic validation
  if (!rule.name) {
    errors.push("Rule name is required");
  }

  if (!rule.type) {
    errors.push("Rule type is required");
  }

  // Type-specific validation
  switch (rule.type) {
    case "simple":
    case "javascript":
      if (!rule.condition) {
        errors.push("Condition is required for simple/javascript rules");
      }
      break;

    case "composite":
      const compositeRule = rule as CompositeRule;
      if (!compositeRule.operator) {
        errors.push("Operator is required for composite rules");
      }
      if (!compositeRule.rules || compositeRule.rules.length === 0) {
        errors.push("Child rules are required for composite rules");
      }
      break;

    case "custom":
      if (!rule.evaluator) {
        errors.push("Evaluator is required for custom rules");
      }
      break;
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Create a simple rule
 */
export function createSimpleRule(
  name: string,
  condition: string,
  options: Partial<Rule> = {},
): Rule {
  return {
    name,
    type: "simple",
    condition,
    enabled: true,
    priority: 0,
    ...options,
  };
}

/**
 * Create a JavaScript rule
 */
export function createJavaScriptRule(
  name: string,
  condition: string,
  options: Partial<Rule> = {},
): Rule {
  return {
    name,
    type: "javascript",
    condition,
    enabled: true,
    priority: 0,
    ...options,
  };
}

/**
 * Create a composite rule
 */
export function createCompositeRule(
  name: string,
  operator: "AND" | "OR" | "NOT",
  rules: Rule[],
  options: Partial<CompositeRule> = {},
): CompositeRule {
  return {
    name,
    type: "composite",
    operator,
    rules,
    enabled: true,
    priority: 0,
    ...options,
  };
}

/**
 * Create a custom rule
 */
export function createCustomRule(
  name: string,
  evaluator: RuleEvaluator,
  options: Partial<Rule> = {},
): Rule {
  return {
    name,
    type: "custom",
    evaluator,
    enabled: true,
    priority: 0,
    ...options,
  };
}
