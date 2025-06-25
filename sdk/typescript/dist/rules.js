/**
 * Rules API client for Circuit Breaker TypeScript SDK
 * Implements client-side rule evaluation for immediate feedback
 */
// ============================================================================
// Rule Builder
// ============================================================================
export class RuleBuilder {
    /**
     * Create a field exists rule
     */
    static fieldExists(id, description, field) {
        return {
            id,
            name: `Field Exists: ${field}`,
            description,
            condition: {
                type: "FieldExists",
                field,
            },
        };
    }
    /**
     * Create a field equals rule
     */
    static fieldEquals(id, description, field, value) {
        return {
            id,
            name: `Field Equals: ${field} = ${value}`,
            description,
            condition: {
                type: "FieldEquals",
                field,
                value,
            },
        };
    }
    /**
     * Create a field greater than rule
     */
    static fieldGreaterThan(id, description, field, value) {
        return {
            id,
            name: `Field Greater Than: ${field} > ${value}`,
            description,
            condition: {
                type: "FieldGreaterThan",
                field,
                value,
            },
        };
    }
    /**
     * Create a field less than rule
     */
    static fieldLessThan(id, description, field, value) {
        return {
            id,
            name: `Field Less Than: ${field} < ${value}`,
            description,
            condition: {
                type: "FieldLessThan",
                field,
                value,
            },
        };
    }
    /**
     * Create a field contains rule
     */
    static fieldContains(id, description, field, value) {
        return {
            id,
            name: `Field Contains: ${field} contains "${value}"`,
            description,
            condition: {
                type: "FieldContains",
                field,
                value,
            },
        };
    }
    /**
     * Create an AND rule combining multiple conditions
     */
    static and(id, description, conditions) {
        return {
            id,
            name: `AND: ${conditions.map((c) => c.name).join(" AND ")}`,
            description,
            condition: {
                type: "And",
                conditions: conditions.map((c) => c.condition),
            },
        };
    }
    /**
     * Create an OR rule combining multiple conditions
     */
    static or(id, description, conditions) {
        return {
            id,
            name: `OR: ${conditions.map((c) => c.name).join(" OR ")}`,
            description,
            condition: {
                type: "Or",
                conditions: conditions.map((c) => c.condition),
            },
        };
    }
    /**
     * Create a NOT rule that inverts a condition
     */
    static not(id, description, condition) {
        return {
            id,
            name: `NOT: ${condition.name}`,
            description,
            condition: {
                type: "Not",
                conditions: [condition.condition],
            },
        };
    }
}
// ============================================================================
// Client-Side Rule Engine
// ============================================================================
export class ClientRuleEngine {
    /**
     * Client-side rule evaluation for immediate UI feedback
     * Note: This should always be validated on the server for authoritative results
     */
    static evaluateRule(rule, tokenData, tokenMetadata = {}) {
        return this.evaluateCondition(rule.condition, tokenData, tokenMetadata, rule.id);
    }
    /**
     * Evaluate multiple rules against the same data
     */
    static evaluateRules(rules, tokenData, tokenMetadata = {}) {
        return rules.map((rule) => this.evaluateRule(rule, tokenData, tokenMetadata));
    }
    /**
     * Check if all rules pass
     */
    static allRulesPass(rules, tokenData, tokenMetadata = {}) {
        return this.evaluateRules(rules, tokenData, tokenMetadata).every((result) => result.passed);
    }
    /**
     * Check if any rule passes
     */
    static anyRulesPasses(rules, tokenData, tokenMetadata = {}) {
        return this.evaluateRules(rules, tokenData, tokenMetadata).some((result) => result.passed);
    }
    static evaluateCondition(condition, data, metadata, ruleId) {
        const combinedData = { ...data, ...metadata };
        switch (condition.type) {
            case "FieldExists":
                const exists = combinedData[condition.field] !== undefined &&
                    combinedData[condition.field] !== null;
                return {
                    rule_id: ruleId,
                    passed: exists,
                    reason: exists
                        ? `Field '${condition.field}' exists`
                        : `Field '${condition.field}' does not exist`,
                    details: {
                        field: condition.field,
                        value: combinedData[condition.field],
                    },
                };
            case "FieldEquals":
                const fieldValue = combinedData[condition.field];
                const equals = fieldValue === condition.value;
                return {
                    rule_id: ruleId,
                    passed: equals,
                    reason: equals
                        ? `Field '${condition.field}' equals ${condition.value}`
                        : `Field '${condition.field}' (${fieldValue}) does not equal ${condition.value}`,
                    details: {
                        field: condition.field,
                        expected: condition.value,
                        actual: fieldValue,
                    },
                };
            case "FieldGreaterThan":
                const numValue = Number(combinedData[condition.field]);
                const greater = !isNaN(numValue) && numValue > condition.value;
                return {
                    rule_id: ruleId,
                    passed: greater,
                    reason: greater
                        ? `Field '${condition.field}' (${numValue}) is greater than ${condition.value}`
                        : `Field '${condition.field}' (${numValue}) is not greater than ${condition.value}`,
                    details: {
                        field: condition.field,
                        threshold: condition.value,
                        actual: numValue,
                    },
                };
            case "FieldLessThan":
                const numValueLt = Number(combinedData[condition.field]);
                const less = !isNaN(numValueLt) && numValueLt < condition.value;
                return {
                    rule_id: ruleId,
                    passed: less,
                    reason: less
                        ? `Field '${condition.field}' (${numValueLt}) is less than ${condition.value}`
                        : `Field '${condition.field}' (${numValueLt}) is not less than ${condition.value}`,
                    details: {
                        field: condition.field,
                        threshold: condition.value,
                        actual: numValueLt,
                    },
                };
            case "FieldContains":
                const strValue = String(combinedData[condition.field] || "");
                const contains = strValue.includes(String(condition.value));
                return {
                    rule_id: ruleId,
                    passed: contains,
                    reason: contains
                        ? `Field '${condition.field}' contains "${condition.value}"`
                        : `Field '${condition.field}' does not contain "${condition.value}"`,
                    details: {
                        field: condition.field,
                        searchValue: condition.value,
                        actualValue: strValue,
                    },
                };
            case "And":
                const andResults = condition.conditions.map((c) => this.evaluateCondition(c, data, metadata, ruleId));
                const allPassed = andResults.every((r) => r.passed);
                return {
                    rule_id: ruleId,
                    passed: allPassed,
                    reason: allPassed
                        ? "All AND conditions passed"
                        : "One or more AND conditions failed",
                    details: { subResults: andResults },
                };
            case "Or":
                const orResults = condition.conditions.map((c) => this.evaluateCondition(c, data, metadata, ruleId));
                const anyPassed = orResults.some((r) => r.passed);
                return {
                    rule_id: ruleId,
                    passed: anyPassed,
                    reason: anyPassed
                        ? "At least one OR condition passed"
                        : "All OR conditions failed",
                    details: { subResults: orResults },
                };
            case "Not":
                if (!condition.conditions || condition.conditions.length === 0) {
                    throw new Error("Not condition requires exactly one sub-condition");
                }
                const notResult = this.evaluateCondition(condition.conditions[0], data, metadata, ruleId);
                return {
                    rule_id: ruleId,
                    passed: !notResult.passed,
                    reason: notResult.passed
                        ? "NOT condition failed (inner condition passed)"
                        : "NOT condition passed (inner condition failed)",
                    details: { innerResult: notResult },
                };
            default:
                return {
                    rule_id: ruleId,
                    passed: false,
                    reason: `Unknown rule type: ${condition.type}`,
                    details: { condition },
                };
        }
    }
}
// ============================================================================
// Rule Client (Legacy API Support)
// ============================================================================
export class RuleClient {
    constructor(_client) {
        this._client = _client;
    }
    /**
     * Create a new rule
     * Note: Server-side rules are not yet implemented in the GraphQL schema
     */
    async create(_input) {
        throw new Error("Server-side rule creation is not yet implemented in the current GraphQL schema. " +
            "Use client-side rule evaluation with RuleBuilder and ClientRuleEngine instead.");
    }
    /**
     * Get a rule by ID
     */
    async get(_id) {
        throw new Error("Server-side rule retrieval is not yet implemented in the GraphQL schema. " +
            "Use client-side rules instead.");
    }
    /**
     * List all rules
     */
    async list() {
        console.warn("Server-side rule listing is not yet implemented in the GraphQL schema.");
        return [];
    }
    /**
     * Update a rule
     */
    async update(_id, _updates) {
        throw new Error("Server-side rule updates are not yet implemented in the GraphQL schema.");
    }
    /**
     * Delete a rule
     */
    async delete(_id) {
        throw new Error("Server-side rule deletion is not yet implemented in the GraphQL schema.");
    }
    /**
     * Evaluate a rule against data
     */
    async evaluate(_id, _data) {
        throw new Error("Server-side rule evaluation is not yet implemented. " +
            "Use ClientRuleEngine.evaluateRule() for client-side evaluation.");
    }
}
// ============================================================================
// Legacy Builder (For Backward Compatibility)
// ============================================================================
export class LegacyRuleBuilder {
    constructor() {
        this.rule = {
            definition: {
                conditions: [],
                actions: [],
            },
        };
    }
    /**
     * Set rule name
     */
    setName(name) {
        this.rule.name = name;
        return this;
    }
    /**
     * Set rule description
     */
    setDescription(description) {
        this.rule.description = description;
        return this;
    }
    /**
     * Add greater than condition
     */
    greaterThan(field, value) {
        if (!this.rule.definition) {
            this.rule.definition = { conditions: [], actions: [] };
        }
        this.rule.definition.conditions.push({
            field,
            operator: "greater_than",
            value,
        });
        return this;
    }
    /**
     * Add equals condition
     */
    equals(field, value) {
        if (!this.rule.definition) {
            this.rule.definition = { conditions: [], actions: [] };
        }
        this.rule.definition.conditions.push({
            field,
            operator: "equals",
            value,
        });
        return this;
    }
    /**
     * Set combinator for conditions
     */
    setCombinator(combinator) {
        if (!this.rule.definition) {
            this.rule.definition = { conditions: [], actions: [] };
        }
        this.rule.definition.combinator = combinator;
        return this;
    }
    /**
     * Add webhook action
     */
    webhook(url, method) {
        if (!this.rule.definition) {
            this.rule.definition = { conditions: [], actions: [] };
        }
        this.rule.definition.actions.push({
            type: "webhook",
            config: { url, method },
        });
        return this;
    }
    /**
     * Add log action
     */
    log(level, message) {
        if (!this.rule.definition) {
            this.rule.definition = { conditions: [], actions: [] };
        }
        this.rule.definition.actions.push({
            type: "log",
            config: { level, message },
        });
        return this;
    }
    /**
     * Build the rule definition
     */
    build() {
        if (!this.rule.name) {
            throw new Error("Rule name is required");
        }
        if (!this.rule.definition || this.rule.definition.conditions.length === 0) {
            throw new Error("Rule must have at least one condition");
        }
        console.warn("Legacy rule creation is not yet supported on the server. " +
            "Consider using the new RuleBuilder and ClientRuleEngine for client-side evaluation.");
        return this.rule;
    }
}
// ============================================================================
// Convenience Functions
// ============================================================================
/**
 * Create a new rule builder (modern pattern)
 */
export const createRule = RuleBuilder;
/**
 * Create a new legacy rule builder (backward compatibility)
 */
export function createLegacyRule(name) {
    return new LegacyRuleBuilder().setName(name);
}
/**
 * Create a client-side rule engine instance
 */
export function createRuleEngine() {
    return ClientRuleEngine;
}
/**
 * Evaluate a single rule against data
 */
export function evaluateRule(rule, data, metadata = {}) {
    return ClientRuleEngine.evaluateRule(rule, data, metadata);
}
/**
 * Create common field validation rules
 */
export const CommonRules = {
    /**
     * Required field rule
     */
    required: (field) => RuleBuilder.fieldExists("required_" + field, `${field} is required`, field),
    /**
     * Minimum value rule
     */
    minValue: (field, min) => RuleBuilder.fieldGreaterThan(`min_${field}`, `${field} must be at least ${min}`, field, min - 0.01),
    /**
     * Maximum value rule
     */
    maxValue: (field, max) => RuleBuilder.fieldLessThan(`max_${field}`, `${field} must be at most ${max}`, field, max + 0.01),
    /**
     * Non-empty string rule
     */
    nonEmpty: (field) => RuleBuilder.and(`non_empty_${field}`, `${field} must not be empty`, [
        RuleBuilder.fieldExists(`${field}_exists`, `${field} exists`, field),
        RuleBuilder.not(`${field}_not_empty`, `${field} is not empty`, RuleBuilder.fieldEquals(`${field}_empty`, `${field} is empty`, field, "")),
    ]),
    /**
     * Email format rule (simple check)
     */
    emailFormat: (field) => RuleBuilder.fieldContains(`email_${field}`, `${field} must be a valid email`, field, "@"),
};
//# sourceMappingURL=rules.js.map