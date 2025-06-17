/**
 * Rules module for Circuit Breaker TypeScript SDK
 *
 * This module provides comprehensive rule management functionality including
 * rule evaluation, validation, CRUD operations, and fluent builders.
 */

// Core rules engine
export {
  RulesEngine,
  createRulesEngine,
  validateRuleDefinition,
  createSimpleRule,
  createJavaScriptRule,
  createCompositeRule,
  createCustomRule,
  // Types
  RuleCreateInput,
  RuleUpdateInput,
  RuleSearchOptions,
  RuleStats,
  RuleWithStats,
  RuleEvaluationOptions,
  BatchRuleEvaluationInput,
  BatchRuleEvaluationResult,
  RuleHealthStatus,
} from "./engine.js";

// Rule builder and utilities
export {
  RuleBuilder,
  SimpleRuleBuilder,
  JavaScriptRuleBuilder,
  CompositeRuleBuilder,
  CustomRuleBuilder,
  RuleBuilderResult,
  BatchRuleBuilder,
  ConditionalGroupBuilder,
  RuleChainBuilder,
  createRuleBuilder,
  createRuleTemplate,
  CommonTemplates,
  // Types
  RuleBuilderOptions,
  RuleTemplate,
  ConditionalRuleGroup,
  RuleChain,
} from "./builder.js";

// Re-export core types that are commonly used with rules
export type {
  Rule,
  RuleType,
  CompositeRule,
  RuleContext,
  RuleEvaluationResult,
  RuleResult,
  RuleValidationResult,
  RuleEvaluator,
  RulesConfig,
} from "../core/types.js";

// Re-export relevant error types
export {
  RuleError,
  RuleEvaluationError,
  RuleNotFoundError,
  RuleValidationError,
  RuleTimeoutError,
} from "../core/errors.js";
