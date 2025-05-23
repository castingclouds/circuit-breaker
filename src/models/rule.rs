// Rules engine for token transition gating

//! # Rules Engine Module
//! 
//! This module defines the rules engine that enables complex condition evaluation
//! for token transitions. Rules can evaluate token metadata and data to determine
//! if a transition should be allowed.
//! 
//! ## Key Concepts
//! 
//! - **Rule**: A single evaluatable condition with an ID and description
//! - **RuleCondition**: The actual logic - field checks, logical operations, etc.
//! - **RuleEvaluationResult**: Detailed results of rule evaluation for debugging
//! 
//! ## Rust Learning Notes:
//! 
//! ### Recursive Enums
//! The `RuleCondition` enum is recursive - `And` and `Or` variants contain
//! vectors of `Rule` structs, which themselves contain `RuleCondition` enums.
//! This enables arbitrarily complex logical expressions.
//! 
//! ### Serde Tag for JSON Serialization
//! The `#[serde(tag = "type")]` attribute creates "tagged union" JSON.
//! Instead of nested objects, it creates flat objects with a "type" field:
//! `{"type": "FieldEquals", "field": "status", "value": "approved"}`

use serde::{Deserialize, Serialize};
use super::token::TokenMetadata;

/// A single rule that can be evaluated against token state
/// 
/// Rules are the building blocks of the condition evaluation system.
/// Each rule has a unique ID, human-readable description, and a condition
/// that defines the actual evaluation logic.
/// 
/// ## Examples:
/// 
/// **Simple field check**:
/// ```
/// use circuit_breaker::models::{Rule, RuleCondition};
/// 
/// Rule {
///     id: "has_reviewer".to_string(),
///     description: "Document must have an assigned reviewer".to_string(),
///     condition: RuleCondition::FieldExists { field: "reviewer".to_string() }
/// };
/// ```
/// 
/// **Complex logical expression**:
/// ```
/// use circuit_breaker::models::{Rule, RuleCondition};
/// 
/// Rule {
///     id: "deployment_ready".to_string(),
///     description: "Ready for deployment".to_string(),
///     condition: RuleCondition::And {
///         rules: vec![
///             Rule { id: "tests".to_string(), description: "tests pass".to_string(), condition: RuleCondition::FieldExists { field: "test_passed".to_string() } },
///             Rule { id: "security".to_string(), description: "security approved".to_string(), condition: RuleCondition::FieldExists { field: "security_approved".to_string() } }
///         ]
///     }
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier for this rule
    /// Used for referencing rules in transitions and debugging
    pub id: String,
    
    /// Human-readable description of what this rule checks
    /// Useful for documentation and error messages
    pub description: String,
    
    /// The actual condition logic to evaluate
    pub condition: RuleCondition,
}

/// Different types of conditions that can be evaluated
/// 
/// This enum represents all the different ways we can evaluate token state.
/// New condition types can be added here as needed.
/// 
/// ## Rust Learning Notes:
/// 
/// ### Tagged Union with Serde
/// The `#[serde(tag = "type")]` creates a "tagged union" in JSON:
/// - `FieldEquals` becomes `{"type": "FieldEquals", "field": "...", "value": "..."}`
/// - `And` becomes `{"type": "And", "rules": [...]}`
/// 
/// This makes the JSON more readable and easier to work with from GraphQL.
/// 
/// ### Box<Rule> for Recursion
/// The `Not` variant uses `Box<Rule>` because Rust enums must have a known size.
/// Since `Rule` contains `RuleCondition` which contains `Rule`, we need `Box`
/// to break the infinite size chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleCondition {
    /// Check if a metadata or data field exists (regardless of value)
    /// 
    /// This is useful for checking if required fields are present.
    /// Checks both token.metadata and token.data.
    /// 
    /// Example: `{"type": "FieldExists", "field": "reviewer"}`
    FieldExists {
        field: String,
    },
    
    /// Check if a field has a specific value
    /// 
    /// Supports any JSON value - strings, numbers, booleans, objects, arrays.
    /// Uses exact equality comparison.
    /// 
    /// Example: `{"type": "FieldEquals", "field": "status", "value": "approved"}`
    FieldEquals {
        field: String,
        value: serde_json::Value,
    },
    
    /// Check if a numeric field is greater than a threshold
    /// 
    /// The field value must be convertible to f64 for comparison.
    /// Non-numeric fields will fail this check.
    /// 
    /// Example: `{"type": "FieldGreaterThan", "field": "score", "value": 85.0}`
    FieldGreaterThan {
        field: String,
        value: f64,
    },
    
    /// Check if a numeric field is less than a threshold
    /// 
    /// Similar to FieldGreaterThan but for upper bounds.
    /// 
    /// Example: `{"type": "FieldLessThan", "field": "risk_score", "value": 50.0}`
    FieldLessThan {
        field: String,
        value: f64,
    },
    
    /// Check if a string field contains a substring
    /// 
    /// Case-sensitive substring search. The field must be a string.
    /// 
    /// Example: `{"type": "FieldContains", "field": "tags", "substring": "urgent"}`
    FieldContains {
        field: String,
        substring: String,
    },
    
    /// Logical AND - all nested rules must pass
    /// 
    /// This is recursive - each rule in the vector can itself be And/Or/etc.
    /// Empty rules vector evaluates to true.
    /// 
    /// Example: `{"type": "And", "rules": [rule1, rule2, rule3]}`
    And {
        rules: Vec<Rule>,
    },
    
    /// Logical OR - at least one nested rule must pass  
    /// 
    /// Recursive like And. Empty rules vector evaluates to false.
    /// 
    /// Example: `{"type": "Or", "rules": [rule1, rule2]}`
    Or {
        rules: Vec<Rule>,
    },
    
    /// Logical NOT - nested rule must fail for this to pass
    /// 
    /// Uses Box because of the recursive type issue.
    /// 
    /// Example: `{"type": "Not", "rule": {...}}`
    Not {
        rule: Box<Rule>,
    },
    
    /// Custom JavaScript expression for complex logic (future)
    /// 
    /// This is a placeholder for future WASM/JavaScript integration.
    /// Would allow arbitrary expressions like "metadata.score > data.threshold * 1.5"
    /// 
    /// Example: `{"type": "Expression", "script": "metadata.score > data.threshold"}`
    Expression {
        script: String,
    },
}

/// Detailed results of rule evaluation
/// 
/// This provides comprehensive information about why a rule passed or failed.
/// Useful for debugging and providing feedback to users.
/// 
/// ## Rust Learning Notes:
/// 
/// ### Vec<(String, bool)> for Results
/// We use a vector of tuples to store rule ID and pass/fail status.
/// This preserves the order of evaluation and makes it easy to iterate.
#[derive(Debug, Clone)]
pub struct RuleEvaluationResult {
    /// ID of the rule that was evaluated
    pub rule_id: String,
    
    /// Whether the overall rule evaluation passed
    pub passed: bool,
    
    /// Detailed results for each sub-rule (for And/Or conditions)
    /// Format: (rule_id, passed)
    pub sub_results: Vec<(String, bool)>,
    
    /// Human-readable explanation of the result
    pub explanation: String,
}

impl Rule {
    /// Evaluate this rule against token metadata and data
    /// 
    /// This is the main entry point for rule evaluation. It evaluates the
    /// rule's condition against the provided token state.
    /// 
    /// ## Parameters
    /// - `metadata`: Token's key-value metadata HashMap
    /// - `data`: Token's JSON data object
    /// 
    /// ## Returns
    /// `true` if the rule passes, `false` if it fails
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Reference Parameters
    /// Both parameters are references (`&`) because we only need to read them.
    /// This avoids unnecessary cloning of potentially large data structures.
    pub fn evaluate(&self, metadata: &TokenMetadata, data: &serde_json::Value) -> bool {
        self.condition.evaluate(metadata, data)
    }
    
    /// Get detailed evaluation results for debugging
    /// 
    /// This provides comprehensive information about the evaluation process,
    /// including which sub-rules passed/failed and explanatory text.
    /// 
    /// Useful for building UIs that show why transitions are/aren't available.
    pub fn evaluate_detailed(&self, metadata: &TokenMetadata, data: &serde_json::Value) -> RuleEvaluationResult {
        let passed = self.evaluate(metadata, data);
        let (sub_results, explanation) = self.condition.evaluate_detailed(metadata, data);
        
        RuleEvaluationResult {
            rule_id: self.id.clone(),
            passed,
            sub_results,
            explanation,
        }
    }
}

impl RuleCondition {
    /// Evaluate the condition against token state
    /// 
    /// This method contains the core evaluation logic for each condition type.
    /// It's called recursively for complex logical expressions.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Pattern Matching on Enums
    /// The `match` expression handles each variant of the enum differently.
    /// Rust ensures we handle all variants - if we add a new condition type,
    /// the compiler will force us to add a case here.
    /// 
    /// ### Option Chaining with and_then
    /// We use `.and_then()` for chaining Option operations. This is like
    /// flatMap in other languages - it only continues if the previous step
    /// returned Some(value).
    pub fn evaluate(&self, metadata: &TokenMetadata, data: &serde_json::Value) -> bool {
        match self {
            RuleCondition::FieldExists { field } => {
                // Check both metadata HashMap and data JSON object
                metadata.contains_key(field) || 
                data.get(field).is_some()
            },
            
            RuleCondition::FieldEquals { field, value } => {
                // Check metadata first, then data
                metadata.get(field) == Some(value) ||
                data.get(field) == Some(value)
            },
            
            RuleCondition::FieldGreaterThan { field, value } => {
                // Try to get numeric value from either metadata or data
                let field_value = metadata.get(field)
                    .or_else(|| data.get(field))  // Try data if not in metadata
                    .and_then(|v| v.as_f64());    // Convert to f64 if possible
                
                // Compare if we got a valid number
                field_value.map_or(false, |v| v > *value)
            },
            
            RuleCondition::FieldLessThan { field, value } => {
                let field_value = metadata.get(field)
                    .or_else(|| data.get(field))
                    .and_then(|v| v.as_f64());
                
                field_value.map_or(false, |v| v < *value)
            },
            
            RuleCondition::FieldContains { field, substring } => {
                // Try to get string value from either metadata or data
                let field_value = metadata.get(field)
                    .or_else(|| data.get(field))
                    .and_then(|v| v.as_str());   // Convert to &str if possible
                
                // Check substring if we got a valid string
                field_value.map_or(false, |v| v.contains(substring))
            },
            
            RuleCondition::And { rules } => {
                // All rules must pass - use iterator's all() method
                rules.iter().all(|rule| rule.evaluate(metadata, data))
            },
            
            RuleCondition::Or { rules } => {
                // At least one rule must pass - use iterator's any() method
                rules.iter().any(|rule| rule.evaluate(metadata, data))
            },
            
            RuleCondition::Not { rule } => {
                // Invert the result of the nested rule
                !rule.evaluate(metadata, data)
            },
            
            RuleCondition::Expression { script: _ } => {
                // TODO: Implement JavaScript/WASM evaluation
                // For now, always return false as a safe default
                // In the future, this could use a JS engine or WASM module
                false
            },
        }
    }
    
    /// Get detailed evaluation results with explanations
    /// 
    /// Returns a tuple of (sub_results, explanation) for building detailed
    /// evaluation reports.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Tuple Return Types
    /// We return `(Vec<(String, bool)>, String)` to provide both structured
    /// data and human-readable text. This avoids needing a custom struct.
    fn evaluate_detailed(&self, metadata: &TokenMetadata, data: &serde_json::Value) -> (Vec<(String, bool)>, String) {
        match self {
            RuleCondition::FieldExists { field } => {
                let exists = metadata.contains_key(field) || data.get(field).is_some();
                let explanation = if exists {
                    format!("Field '{}' exists", field)
                } else {
                    format!("Field '{}' does not exist", field)
                };
                (vec![], explanation)
            },
            
            RuleCondition::FieldEquals { field, value } => {
                let matches = metadata.get(field) == Some(value) || data.get(field) == Some(value);
                let explanation = if matches {
                    format!("Field '{}' equals {:?}", field, value)
                } else {
                    format!("Field '{}' does not equal {:?}", field, value)
                };
                (vec![], explanation)
            },
            
            RuleCondition::FieldGreaterThan { field, value } => {
                let field_value = metadata.get(field).or_else(|| data.get(field)).and_then(|v| v.as_f64());
                let explanation = match field_value {
                    Some(v) if v > *value => format!("Field '{}' ({}) > {}", field, v, value),
                    Some(v) => format!("Field '{}' ({}) <= {}", field, v, value),
                    None => format!("Field '{}' is not a number", field),
                };
                (vec![], explanation)
            },
            
            RuleCondition::FieldLessThan { field, value } => {
                let field_value = metadata.get(field).or_else(|| data.get(field)).and_then(|v| v.as_f64());
                let explanation = match field_value {
                    Some(v) if v < *value => format!("Field '{}' ({}) < {}", field, v, value),
                    Some(v) => format!("Field '{}' ({}) >= {}", field, v, value),
                    None => format!("Field '{}' is not a number", field),
                };
                (vec![], explanation)
            },
            
            RuleCondition::FieldContains { field, substring } => {
                let field_value = metadata.get(field).or_else(|| data.get(field)).and_then(|v| v.as_str());
                let explanation = match field_value {
                    Some(v) if v.contains(substring) => format!("Field '{}' contains '{}'", field, substring),
                    Some(_) => format!("Field '{}' does not contain '{}'", field, substring),
                    None => format!("Field '{}' is not a string", field),
                };
                (vec![], explanation)
            },
            
            RuleCondition::And { rules } => {
                let sub_results: Vec<(String, bool)> = rules.iter()
                    .map(|rule| (rule.id.clone(), rule.evaluate(metadata, data)))
                    .collect();
                let explanation = format!("AND: {} of {} rules passed", 
                    sub_results.iter().filter(|(_, passed)| *passed).count(),
                    sub_results.len()
                );
                (sub_results, explanation)
            },
            
            RuleCondition::Or { rules } => {
                let sub_results: Vec<(String, bool)> = rules.iter()
                    .map(|rule| (rule.id.clone(), rule.evaluate(metadata, data)))
                    .collect();
                let explanation = format!("OR: {} of {} rules passed", 
                    sub_results.iter().filter(|(_, passed)| *passed).count(),
                    sub_results.len()
                );
                (sub_results, explanation)
            },
            
            RuleCondition::Not { rule } => {
                let passed = rule.evaluate(metadata, data);
                let explanation = format!("NOT: nested rule '{}' {}", 
                    rule.id, 
                    if passed { "passed (so NOT fails)" } else { "failed (so NOT passes)" }
                );
                (vec![(rule.id.clone(), !passed)], explanation)
            },
            
            RuleCondition::Expression { script } => {
                (vec![], format!("Expression '{}' not implemented", script))
            },
        }
    }
}

// Builder methods for easier rule construction
impl Rule {
    /// Create a simple field exists rule
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::field_exists("has_reviewer", "reviewer");
    /// ```
    pub fn field_exists(id: &str, field: &str) -> Self {
        Rule {
            id: id.to_string(),
            description: format!("Field '{}' must exist", field),
            condition: RuleCondition::FieldExists {
                field: field.to_string(),
            },
        }
    }
    
    /// Create a simple field equals rule
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::field_equals("status_approved", "status", serde_json::json!("approved"));
    /// ```
    pub fn field_equals(id: &str, field: &str, value: serde_json::Value) -> Self {
        Rule {
            id: id.to_string(),
            description: format!("Field '{}' must equal {:?}", field, value),
            condition: RuleCondition::FieldEquals {
                field: field.to_string(),
                value,
            },
        }
    }
    
    /// Create a field greater than rule
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::field_greater_than("high_score", "score", 85.0);
    /// ```
    pub fn field_greater_than(id: &str, field: &str, value: f64) -> Self {
        Rule {
            id: id.to_string(),
            description: format!("Field '{}' must be greater than {}", field, value),
            condition: RuleCondition::FieldGreaterThan {
                field: field.to_string(),
                value,
            },
        }
    }
    
    /// Create an AND combination of rules
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::and("ready_to_deploy", "Ready for deployment", vec![
    ///     Rule::field_equals("tests", "test_status", serde_json::json!("passed")),
    ///     Rule::field_equals("security", "security_status", serde_json::json!("approved")),
    /// ]);
    /// ```
    pub fn and(id: &str, description: &str, rules: Vec<Rule>) -> Self {
        Rule {
            id: id.to_string(),
            description: description.to_string(),
            condition: RuleCondition::And { rules },
        }
    }
    
    /// Create an OR combination of rules
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::or("approval_or_override", "Has approval or override", vec![
    ///     Rule::field_equals("approved", "manager_approved", serde_json::json!(true)),
    ///     Rule::field_equals("override", "emergency_override", serde_json::json!(true)),
    /// ]);
    /// ```
    pub fn or(id: &str, description: &str, rules: Vec<Rule>) -> Self {
        Rule {
            id: id.to_string(),
            description: description.to_string(),
            condition: RuleCondition::Or { rules },
        }
    }
    
    /// Create a NOT rule
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::models::Rule;
    /// 
    /// let rule = Rule::not("not_flagged", "Not flagged for review", 
    ///     Rule::field_equals("flagged", "review_flag", serde_json::json!(true))
    /// );
    /// ```
    pub fn not(id: &str, description: &str, rule: Rule) -> Self {
        Rule {
            id: id.to_string(),
            description: description.to_string(),
            condition: RuleCondition::Not {
                rule: Box::new(rule),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_field_exists_rule() {
        let rule = Rule::field_exists("has_reviewer", "reviewer");
        
        // Test with metadata
        let mut metadata = HashMap::new();
        metadata.insert("reviewer".to_string(), serde_json::json!("alice"));
        let data = serde_json::json!({});
        
        assert!(rule.evaluate(&metadata, &data));
        
        // Test with data instead of metadata
        let empty_metadata = HashMap::new();
        let data_with_reviewer = serde_json::json!({"reviewer": "bob"});
        
        assert!(rule.evaluate(&empty_metadata, &data_with_reviewer));
        
        // Test with neither
        assert!(!rule.evaluate(&empty_metadata, &data));
    }
    
    #[test]
    fn test_field_equals_rule() {
        let rule = Rule::field_equals("status_approved", "status", serde_json::json!("approved"));
        
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), serde_json::json!("approved"));
        let data = serde_json::json!({});
        
        assert!(rule.evaluate(&metadata, &data));
        
        // Test wrong value
        metadata.insert("status".to_string(), serde_json::json!("pending"));
        assert!(!rule.evaluate(&metadata, &data));
    }
    
    #[test]
    fn test_complex_and_rule() {
        let rule = Rule::and(
            "deployment_ready",
            "Ready for deployment",
            vec![
                Rule::field_equals("tests", "test_status", serde_json::json!("passed")),
                Rule::field_equals("security", "security_status", serde_json::json!("approved")),
            ]
        );
        
        // Both conditions met
        let mut metadata = HashMap::new();
        metadata.insert("test_status".to_string(), serde_json::json!("passed"));
        metadata.insert("security_status".to_string(), serde_json::json!("approved"));
        let data = serde_json::json!({});
        
        assert!(rule.evaluate(&metadata, &data));
        
        // Only one condition met
        metadata.remove("security_status");
        assert!(!rule.evaluate(&metadata, &data));
    }
    
    #[test]
    fn test_complex_or_rule() {
        let rule = Rule::or(
            "approval_or_override",
            "Has approval or override",
            vec![
                Rule::field_equals("approved", "manager_approved", serde_json::json!(true)),
                Rule::field_equals("override", "emergency_override", serde_json::json!(true)),
            ]
        );
        
        // First condition met
        let mut metadata = HashMap::new();
        metadata.insert("manager_approved".to_string(), serde_json::json!(true));
        let data = serde_json::json!({});
        
        assert!(rule.evaluate(&metadata, &data));
        
        // Second condition met instead
        metadata.clear();
        metadata.insert("emergency_override".to_string(), serde_json::json!(true));
        assert!(rule.evaluate(&metadata, &data));
        
        // Neither condition met
        metadata.clear();
        assert!(!rule.evaluate(&metadata, &data));
    }
    
    #[test]
    fn test_not_rule() {
        let rule = Rule::not(
            "not_flagged",
            "Not flagged for review",
            Rule::field_equals("flagged", "review_flag", serde_json::json!(true))
        );
        
        // Not flagged (good)
        let metadata = HashMap::new();
        let data = serde_json::json!({});
        assert!(rule.evaluate(&metadata, &data));
        
        // Flagged (bad)
        let mut metadata = HashMap::new();
        metadata.insert("review_flag".to_string(), serde_json::json!(true));
        assert!(!rule.evaluate(&metadata, &data));
    }
    
    #[test]
    fn test_numeric_comparisons() {
        let rule = Rule::field_greater_than("high_score", "score", 85.0);
        
        // High score
        let mut metadata = HashMap::new();
        metadata.insert("score".to_string(), serde_json::json!(90.5));
        let data = serde_json::json!({});
        assert!(rule.evaluate(&metadata, &data));
        
        // Low score
        metadata.insert("score".to_string(), serde_json::json!(75.0));
        assert!(!rule.evaluate(&metadata, &data));
        
        // Non-numeric value
        metadata.insert("score".to_string(), serde_json::json!("not_a_number"));
        assert!(!rule.evaluate(&metadata, &data));
    }
    
    #[test]
    fn test_detailed_evaluation() {
        let rule = Rule::and(
            "complex_approval",
            "Complex approval logic",
            vec![
                Rule::field_exists("has_content", "content"),
                Rule::field_equals("status_approved", "status", serde_json::json!("approved")),
            ]
        );
        
        let mut metadata = HashMap::new();
        metadata.insert("content".to_string(), serde_json::json!("Hello world"));
        metadata.insert("status".to_string(), serde_json::json!("pending"));
        let data = serde_json::json!({});
        
        let result = rule.evaluate_detailed(&metadata, &data);
        
        assert_eq!(result.rule_id, "complex_approval");
        assert!(!result.passed); // Should fail because status != "approved"
        assert_eq!(result.sub_results.len(), 2);
        assert_eq!(result.sub_results[0].0, "has_content");
        assert!(result.sub_results[0].1); // First rule should pass
        assert_eq!(result.sub_results[1].0, "status_approved");
        assert!(!result.sub_results[1].1); // Second rule should fail
    }
} 