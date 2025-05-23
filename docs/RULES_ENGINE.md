# Rules Engine Documentation

The Circuit Breaker Rules Engine provides sophisticated condition evaluation for token transitions through workflows. This enables complex business logic gating without hardcoding domain-specific rules into the core engine.

## Overview

The rules engine allows you to define complex logical expressions that determine when tokens can transition between places in a workflow. Instead of simple string-based conditions, you can create structured rules that evaluate token metadata and data fields.

## Key Components

### 1. Rule (`Rule`)

A `Rule` represents a single evaluatable condition with:
- **ID**: Unique identifier for the rule
- **Description**: Human-readable explanation
- **Condition**: The actual evaluation logic (`RuleCondition`)

### 2. Rule Conditions (`RuleCondition`)

Different types of conditions that can be evaluated:

#### Basic Field Conditions
- `FieldExists`: Check if a field exists in metadata or data
- `FieldEquals`: Check if a field has a specific value
- `FieldGreaterThan` / `FieldLessThan`: Numeric comparisons
- `FieldContains`: Substring matching in string fields

#### Logical Operations
- `And`: All nested rules must pass
- `Or`: At least one nested rule must pass  
- `Not`: Nested rule must fail

#### Advanced
- `Expression`: Custom JavaScript/WASM expressions (future)

### 3. Rules Engine (`RulesEngine`)

Central service that:
- Manages global rules registry
- Evaluates tokens against transition rules
- Provides detailed evaluation feedback
- Maintains backwards compatibility with string conditions

### 4. Enhanced Transitions

`TransitionDefinition` now supports:
- **Legacy conditions**: String-based conditions (backwards compatible)
- **Structured rules**: New `Vec<Rule>` field for sophisticated evaluation
- **Rule evaluation methods**: Built-in rule checking capabilities

## Usage Examples

### Simple Rules

```rust
// Basic field existence check
let rule = Rule::field_exists("has_reviewer", "reviewer");

// Value equality check
let rule = Rule::field_equals("status_approved", "status", json!("approved"));

// Numeric comparison
let rule = Rule::field_greater_than("high_priority", "priority", 5.0);
```

### Complex Logical Expressions

```rust
// (Rule A && Rule B) || Rule C
let complex_rule = Rule::or(
    "deployment_ready",
    "Ready for deployment",
    vec![
        // Normal criteria (Rule A && Rule B)
        Rule::and(
            "standard_deployment",
            "Standard deployment checks",
            vec![
                Rule::field_equals("tests", "test_status", json!("passed")),
                Rule::field_equals("security", "security_status", json!("approved")),
            ]
        ),
        // Emergency override (Rule C)
        Rule::field_equals("emergency", "emergency_deploy", json!(true)),
    ]
);
```

### Transition with Rules

```rust
let transition = TransitionDefinition::with_rules(
    "deploy_to_production",
    vec!["staging"],
    "production",
    vec![complex_rule]
);
```

### Rules Engine Usage

```rust
// Create engine with common rules
let mut engine = RulesEngine::with_common_rules();

// Register custom rules
engine.register_rule(custom_rule);

// Evaluate token against workflow
let available = engine.available_transitions(&token, &workflow);
let detailed = engine.evaluate_all_transitions(&token, &workflow);

// Check specific transition
let can_fire = engine.can_transition(&token, &transition);
```

## Built-in Common Rules

The rules engine comes with predefined rules for common scenarios:

### Content Validation
- `has_content`: Content field exists
- `has_title`: Title field exists
- `has_description`: Description field exists

### Approval Workflows
- `has_reviewer`: Reviewer assigned
- `has_approver`: Approver assigned
- `status_approved`: Status equals "approved"
- `status_rejected`: Status equals "rejected"
- `status_pending`: Status equals "pending"

### Priority & Urgency
- `high_priority`: Priority > 5
- `critical_priority`: Priority > 8
- `emergency_flag`: Emergency field is true

### Testing & Deployment
- `tests_passed`: Test status is "passed"
- `tests_failed`: Test status is "failed"
- `security_approved`: Security status is "approved"
- `security_flagged`: Security status is "flagged"

### Users & Permissions
- `has_assignee`: Assignee field exists
- `has_creator`: Creator field exists
- `admin_override`: Admin override flag is true

## Rule Evaluation Process

1. **Place Compatibility**: Check if token is in correct place for transition
2. **Structured Rules**: Evaluate all rules in `TransitionDefinition.rules`
3. **Legacy Conditions**: Evaluate string conditions in `TransitionDefinition.conditions`
4. **Final Decision**: All checks must pass for transition to fire

## Important: Structured Rules vs Complete Evaluation

There's an important distinction between different evaluation methods:

### TransitionDefinition Methods (Partial Evaluation)

These methods only evaluate **structured rules** and place compatibility:

```rust
// Only evaluates structured rules - NOT legacy conditions
let can_fire_partial = transition.can_fire_with_token(&token);
let rules_pass = transition.rules_pass(&token);
let result = transition.evaluate_with_token(&token);
```

**What's evaluated:**
- ✅ Place compatibility (`from_places`)
- ✅ Structured rules (`rules` field)
- ❌ Legacy conditions (`conditions` field) - **NOT evaluated**

### RulesEngine Methods (Complete Evaluation)

These methods provide **complete evaluation** including all condition types:

```rust
// Complete evaluation including legacy conditions
let engine = RulesEngine::with_common_rules();
let can_fire_complete = engine.can_transition(&token, &transition);
let all_results = engine.evaluate_all_transitions(&token, &workflow);
```

**What's evaluated:**
- ✅ Place compatibility (`from_places`)
- ✅ Structured rules (`rules` field)
- ✅ Legacy conditions (`conditions` field) - **Fully evaluated**

### Why This Distinction Exists

- **Models Layer**: The `TransitionDefinition` (in `src/models/`) is domain-agnostic and doesn't know about global rule registries
- **Engine Layer**: The `RulesEngine` (in `src/engine/`) has access to global rules needed to resolve legacy string-based conditions
- **Legacy Support**: String conditions require rule name resolution, which only the engine can provide

### Example: Different Results

```rust
// Transition with both structured rules and legacy conditions
let mut transition = TransitionDefinition::with_conditions(
    "complex_approval",
    vec!["draft"],
    "approved",
    vec!["requires_manager_approval".to_string()] // Legacy condition
);
transition.add_rule(Rule::field_exists("has_content", "content"));

// Token that satisfies structured rule but not legacy condition
let mut token = Token::new("test", PlaceId::from("draft"));
token.data = json!({"content": "test content"});

// Partial evaluation (structured rules only) - PASSES
let partial = transition.can_fire_with_token(&token); // true

// Complete evaluation (including legacy) - MAY FAIL
let engine = RulesEngine::with_common_rules();
let complete = engine.can_transition(&token, &transition); // false if legacy condition fails
```

### Best Practices

1. **Use `RulesEngine` methods for authoritative evaluation**
2. **Use `TransitionDefinition` methods only for debugging structured rules**
3. **Document which evaluation method your code uses**
4. **Migrate from legacy conditions to structured rules when possible**

## Detailed Evaluation Results

The engine provides comprehensive feedback:

```rust
pub struct TransitionRuleEvaluation {
    pub transition_id: TransitionId,
    pub place_compatible: bool,
    pub rules_passed: bool,
    pub can_fire: bool,
    pub rule_results: Vec<RuleEvaluationResult>,
    pub explanation: String,
}
```

This enables:
- **Debugging**: Understand why transitions fail
- **User Feedback**: Show users what conditions are missing
- **UI Development**: Build interfaces showing requirements
- **Workflow Analysis**: Optimize workflow logic

## Backwards Compatibility

The rules engine maintains full backwards compatibility:

- **Existing workflows** continue to work unchanged
- **String conditions** are resolved to global rules if available
- **Default behavior** preserved when no matching rule found
- **Gradual migration** supported via `with_conditions_and_rules()`

## Architecture Benefits

### 1. Domain Agnostic
- Rules evaluate generic JSON metadata/data
- No hardcoded business logic in core engine
- Works with any domain (documents, orders, deployments, etc.)

### 2. Composable
- Rules can be combined with AND, OR, NOT operations
- Complex expressions built from simple components
- Reusable rules across multiple workflows

### 3. Debuggable
- Detailed evaluation results show exactly why rules pass/fail
- Hierarchical feedback for complex expressions
- Enables sophisticated workflow debugging tools

### 4. Extensible
- New rule types easily added to `RuleCondition` enum
- Global rule registry for reusable business logic
- Future support for custom expressions (WASM/JavaScript)

### 5. Type Safe
- Rust's type system prevents rule evaluation errors
- Compile-time guarantees about rule structure
- Serde serialization for API integration

## Real-world Example

See `examples/rust/rules_engine_demo.rs` for a complete example showing:

- Complex rule definitions with nested AND/OR logic
- Multiple token scenarios (ready, incomplete, emergency)
- Detailed evaluation feedback and debugging
- Integration with workflow engine

The demo shows an article publishing workflow with rules like:

**Publish Criteria**: `(content exists AND title exists AND reviewer assigned AND status approved AND word count > 500) OR emergency flag`

This demonstrates how complex business logic can be expressed as structured, debuggable rules rather than hardcoded conditions.

## Performance Considerations

- **Short-circuit evaluation**: Place compatibility checked first (cheapest)
- **Rule ordering**: Most restrictive rules should be checked first
- **Global rule caching**: Common rules stored once and reused
- **Lazy evaluation**: OR rules stop at first success, AND rules stop at first failure

## GraphQL Integration

The rules engine integrates with the GraphQL API through:
- Rule evaluation endpoints
- Detailed transition analysis
- Available transitions with reasoning
- Rule management operations

This enables building sophisticated UIs that show users exactly what conditions need to be met for workflow progression. 