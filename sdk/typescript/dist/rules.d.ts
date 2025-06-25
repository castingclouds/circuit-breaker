/**
 * Rules API client for Circuit Breaker TypeScript SDK
 * Implements client-side rule evaluation for immediate feedback
 */
import type { Client } from "./client.js";
export interface Rule {
    id: string;
    name: string;
    description: string;
    condition: RuleCondition;
}
export interface RuleCondition {
    type: "And" | "Or" | "Not" | "FieldExists" | "FieldEquals" | "FieldGreaterThan" | "FieldLessThan" | "FieldContains";
    field?: string;
    value?: any;
    conditions?: RuleCondition[];
}
export interface RuleEvaluationResult {
    rule_id: string;
    passed: boolean;
    reason: string;
    details?: Record<string, any>;
}
export interface RuleCreateInput {
    name: string;
    description?: string;
    type: "simple" | "composite" | "javascript" | "custom";
    definition: {
        conditions: Array<{
            field: string;
            operator: string;
            value: any;
        }>;
        actions: Array<{
            type: string;
            config: Record<string, any>;
        }>;
        combinator?: "and" | "or";
    };
}
export interface LegacyRule {
    id: string;
    name: string;
    description?: string;
    type: string;
    definition: any;
    created_at: string;
    updated_at: string;
}
export declare class RuleBuilder {
    /**
     * Create a field exists rule
     */
    static fieldExists(id: string, description: string, field: string): Rule;
    /**
     * Create a field equals rule
     */
    static fieldEquals(id: string, description: string, field: string, value: any): Rule;
    /**
     * Create a field greater than rule
     */
    static fieldGreaterThan(id: string, description: string, field: string, value: number): Rule;
    /**
     * Create a field less than rule
     */
    static fieldLessThan(id: string, description: string, field: string, value: number): Rule;
    /**
     * Create a field contains rule
     */
    static fieldContains(id: string, description: string, field: string, value: string): Rule;
    /**
     * Create an AND rule combining multiple conditions
     */
    static and(id: string, description: string, conditions: Rule[]): Rule;
    /**
     * Create an OR rule combining multiple conditions
     */
    static or(id: string, description: string, conditions: Rule[]): Rule;
    /**
     * Create a NOT rule that inverts a condition
     */
    static not(id: string, description: string, condition: Rule): Rule;
}
export declare class ClientRuleEngine {
    /**
     * Client-side rule evaluation for immediate UI feedback
     * Note: This should always be validated on the server for authoritative results
     */
    static evaluateRule(rule: Rule, tokenData: any, tokenMetadata?: any): RuleEvaluationResult;
    /**
     * Evaluate multiple rules against the same data
     */
    static evaluateRules(rules: Rule[], tokenData: any, tokenMetadata?: any): RuleEvaluationResult[];
    /**
     * Check if all rules pass
     */
    static allRulesPass(rules: Rule[], tokenData: any, tokenMetadata?: any): boolean;
    /**
     * Check if any rule passes
     */
    static anyRulesPasses(rules: Rule[], tokenData: any, tokenMetadata?: any): boolean;
    private static evaluateCondition;
}
export declare class RuleClient {
    private _client;
    constructor(_client: Client);
    /**
     * Create a new rule
     * Note: Server-side rules are not yet implemented in the GraphQL schema
     */
    create(_input: RuleCreateInput): Promise<LegacyRule>;
    /**
     * Get a rule by ID
     */
    get(_id: string): Promise<LegacyRule>;
    /**
     * List all rules
     */
    list(): Promise<LegacyRule[]>;
    /**
     * Update a rule
     */
    update(_id: string, _updates: Partial<RuleCreateInput>): Promise<LegacyRule>;
    /**
     * Delete a rule
     */
    delete(_id: string): Promise<boolean>;
    /**
     * Evaluate a rule against data
     */
    evaluate(_id: string, _data: Record<string, any>): Promise<RuleEvaluationResult>;
}
export declare class LegacyRuleBuilder {
    private rule;
    /**
     * Set rule name
     */
    setName(name: string): LegacyRuleBuilder;
    /**
     * Set rule description
     */
    setDescription(description: string): LegacyRuleBuilder;
    /**
     * Add greater than condition
     */
    greaterThan(field: string, value: any): LegacyRuleBuilder;
    /**
     * Add equals condition
     */
    equals(field: string, value: any): LegacyRuleBuilder;
    /**
     * Set combinator for conditions
     */
    setCombinator(combinator: "and" | "or"): LegacyRuleBuilder;
    /**
     * Add webhook action
     */
    webhook(url: string, method: string): LegacyRuleBuilder;
    /**
     * Add log action
     */
    log(level: string, message: string): LegacyRuleBuilder;
    /**
     * Build the rule definition
     */
    build(): RuleCreateInput;
}
/**
 * Create a new rule builder (modern pattern)
 */
export declare const createRule: typeof RuleBuilder;
/**
 * Create a new legacy rule builder (backward compatibility)
 */
export declare function createLegacyRule(name: string): LegacyRuleBuilder;
/**
 * Create a client-side rule engine instance
 */
export declare function createRuleEngine(): typeof ClientRuleEngine;
/**
 * Evaluate a single rule against data
 */
export declare function evaluateRule(rule: Rule, data: any, metadata?: any): RuleEvaluationResult;
/**
 * Create common field validation rules
 */
export declare const CommonRules: {
    /**
     * Required field rule
     */
    required: (field: string) => Rule;
    /**
     * Minimum value rule
     */
    minValue: (field: string, min: number) => Rule;
    /**
     * Maximum value rule
     */
    maxValue: (field: string, max: number) => Rule;
    /**
     * Non-empty string rule
     */
    nonEmpty: (field: string) => Rule;
    /**
     * Email format rule (simple check)
     */
    emailFormat: (field: string) => Rule;
};
//# sourceMappingURL=rules.d.ts.map