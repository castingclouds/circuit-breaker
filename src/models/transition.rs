// Transition definitions - workflow state change specifications

//! # Transition Definitions
//! 
//! This module defines how tokens can move between places in a workflow.
//! A `TransitionDefinition` specifies:
//! - Which places a token can come from (source places)
//! - Which place a token will go to (target place)
//! - What conditions must be met for the transition to fire
//! - What rules must be satisfied for the transition to be enabled
//! 
//! ## Petri Net Theory
//! 
//! In Petri Net terminology:
//! - **Input Places**: Places where tokens must be present for transition to fire
//! - **Output Place**: Place where tokens will be created after transition fires
//! - **Conditions**: Additional business logic that must be satisfied
//! - **Rules**: Structured conditions that can be evaluated against token state
//! 
//! ## Rust Learning Notes:
//! 
//! This file demonstrates advanced Rust concepts:
//! - Complex generic functions with multiple type parameters
//! - Trait bounds for flexible APIs
//! - Iterator methods and functional programming
//! - Collection operations (contains, map, collect)

use serde::{Deserialize, Serialize}; // JSON serialization traits
use super::place::{PlaceId, TransitionId}; // Import from sibling module
use super::rule::{Rule, RuleEvaluationResult}; // Import rules engine
use super::token::Token; // Import token for rule evaluation

/// Generic transition definition
/// 
/// Defines how tokens can move from one or more source places to a target place.
/// This is completely domain-agnostic - any workflow can use this structure.
/// 
/// ## Examples:
/// 
/// **Document Review**:
/// - from_places: ["draft"]
/// - to_place: "review"
/// - conditions: ["has_content", "assigned_reviewer"]
/// - rules: [Rule::field_exists("reviewer", "reviewer")]
/// 
/// **Software Deployment**:
/// - from_places: ["tested"] 
/// - to_place: "production"
/// - conditions: ["all_tests_pass", "security_approved"]
/// - rules: [Rule::and("deployment_ready", "Ready for deployment", vec![...])]
/// 
/// **Order Processing**:
/// - from_places: ["cart"]
/// - to_place: "payment_pending"
/// - conditions: ["items_available", "customer_verified"]
/// - rules: [Rule::field_equals("payment_method", "payment_method", json!("credit_card"))]
/// 
/// ## Rust Learning Notes:
/// 
/// ### Derive Macros
/// The derive attributes automatically implement traits:
/// - `Debug`: Enables `println!("{:?}", transition)` for debugging
/// - `Clone`: Allows `transition.clone()` to create copies
/// - `Serialize, Deserialize`: Enables JSON conversion via serde
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDefinition {
    /// Unique identifier for this transition
    /// Examples: "submit", "approve", "deploy", "cancel"
    pub id: TransitionId,
    
    /// Places where tokens must be present for this transition to fire
    /// Can be multiple places for synchronization (join operations)
    /// Examples: ["draft"], ["review", "legal_check"], ["tested", "approved"]
    pub from_places: Vec<PlaceId>,
    
    /// Place where tokens will be created after transition fires
    /// Always exactly one place (Petri nets can split to multiple, but we simplify)
    /// Examples: "review", "approved", "production"
    pub to_place: PlaceId,
    
    /// Legacy business logic conditions (kept for backwards compatibility)
    /// These are domain-specific rules checked by the application
    /// Examples: ["has_content"], ["all_tests_pass", "security_scan_clean"]
    pub conditions: Vec<String>, // Generic condition strings
    
    /// Structured rules that can be evaluated against token state
    /// These provide more sophisticated condition evaluation than simple strings
    /// Rules can check metadata, data fields, and combine with logical operations
    pub rules: Vec<Rule>,
}

/// Results of evaluating structured rules for a transition
/// 
/// This provides comprehensive information about rule evaluation,
/// useful for debugging and providing user feedback.
/// 
/// **Important**: This only includes results from structured rules.
/// Legacy string-based conditions are evaluated separately by the RulesEngine.
#[derive(Debug, Clone)]
pub struct TransitionRuleEvaluation {
    /// ID of the transition that was evaluated
    pub transition_id: TransitionId,
    
    /// Whether the token is in a compatible place for this transition
    pub place_compatible: bool,
    
    /// Whether all structured rules passed evaluation
    /// (Legacy conditions are not included in this evaluation)
    pub rules_passed: bool,
    
    /// Whether the transition can fire based on place and structured rules only
    /// (Legacy conditions are not included - use RulesEngine for complete evaluation)
    pub can_fire: bool,
    
    /// Detailed results for each structured rule
    pub rule_results: Vec<RuleEvaluationResult>,
    
    /// Overall explanation of the evaluation
    pub explanation: String,
}

impl TransitionDefinition {
    /// Create a new transition definition
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Complex Generic Function Signature
    /// This function has three generic type parameters with trait bounds:
    /// - `I: Into<TransitionId>` - id can be &str, String, or TransitionId
    /// - `F: Into<PlaceId>` - each from_place can be &str, String, or PlaceId  
    /// - `T: Into<PlaceId>` - to_place can be &str, String, or PlaceId
    /// 
    /// ### Trait Bounds (the `where` clause)
    /// The `where` clause makes complex generic signatures more readable.
    /// It's equivalent to writing the bounds inline but cleaner for multiple bounds.
    /// 
    /// ### Into<T> Trait
    /// The `Into` trait enables automatic conversions. This means callers can pass:
    /// - String literals: "submit"
    /// - String objects: my_string
    /// - Already-constructed types: TransitionId::from("submit")
    pub fn new<I, F, T>(id: I, from_places: Vec<F>, to_place: T) -> Self 
    where
        I: Into<TransitionId>,  // id parameter can convert to TransitionId
        F: Into<PlaceId>,       // each element of from_places can convert to PlaceId
        T: Into<PlaceId>,       // to_place parameter can convert to PlaceId
    {
        TransitionDefinition {
            // Convert the id parameter to TransitionId
            id: id.into(),
            
            // Convert each element of from_places vector to PlaceId
            // This uses iterator methods - a functional programming approach
            from_places: from_places
                .into_iter()           // Convert vector to iterator (consumes the vector)
                .map(|s| s.into())     // Transform each element using Into trait
                .collect(),            // Collect results back into a Vec<PlaceId>
            
            // Convert the to_place parameter to PlaceId
            to_place: to_place.into(),
            
            // Start with no conditions - can be added later if needed
            conditions: vec![],
            
            // Start with no rules - can be added later if needed
            rules: vec![],
        }
    }

    /// Create a transition with conditions (legacy support)
    /// 
    /// This is a more complete constructor that allows specifying conditions
    /// up front instead of adding them later.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Method Overloading Alternative
    /// Rust doesn't have method overloading like Java/C#. Instead, we create
    /// methods with different names like `new()` vs `with_conditions()`.
    /// 
    /// ### Same Generic Pattern
    /// Uses the same generic parameters as `new()` but adds a conditions parameter.
    /// The Vec<String> doesn't need generics because condition strings are
    /// already in their final form.
    pub fn with_conditions<I, F, T>(
        id: I, 
        from_places: Vec<F>, 
        to_place: T, 
        conditions: Vec<String>  // Conditions are already strings
    ) -> Self 
    where
        I: Into<TransitionId>,
        F: Into<PlaceId>,
        T: Into<PlaceId>,
    {
        TransitionDefinition {
            id: id.into(),
            // Same iterator pattern as new()
            from_places: from_places.into_iter().map(|s| s.into()).collect(),
            to_place: to_place.into(),
            conditions, // Move the conditions vector directly
            rules: vec![], // Start with no rules
        }
    }
    
    /// Create a transition with structured rules
    /// 
    /// This is the new preferred way to create transitions with sophisticated
    /// condition evaluation using the rules engine.
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::{TransitionDefinition, models::Rule};
    /// use serde_json::json;
    /// 
    /// let transition = TransitionDefinition::with_rules(
    ///     "deploy",
    ///     vec!["tested"],
    ///     "production",
    ///     vec![
    ///         Rule::and("deployment_ready", "Ready for deployment", vec![
    ///             Rule::field_equals("tests", "test_status", json!("passed")),
    ///             Rule::field_equals("security", "security_status", json!("approved")),
    ///         ])
    ///     ]
    /// );
    /// ```
    pub fn with_rules<I, F, T>(
        id: I,
        from_places: Vec<F>,
        to_place: T,
        rules: Vec<Rule>
    ) -> Self 
    where
        I: Into<TransitionId>,
        F: Into<PlaceId>,
        T: Into<PlaceId>,
    {
        TransitionDefinition {
            id: id.into(),
            from_places: from_places.into_iter().map(|s| s.into()).collect(),
            to_place: to_place.into(),
            conditions: vec![], // Legacy conditions empty when using rules
            rules,
        }
    }
    
    /// Create a transition with both conditions and rules
    /// 
    /// This allows for gradual migration from string conditions to structured rules.
    /// Both types of conditions must pass for the transition to fire.
    pub fn with_conditions_and_rules<I, F, T>(
        id: I,
        from_places: Vec<F>,
        to_place: T,
        conditions: Vec<String>,
        rules: Vec<Rule>
    ) -> Self 
    where
        I: Into<TransitionId>,
        F: Into<PlaceId>,
        T: Into<PlaceId>,
    {
        TransitionDefinition {
            id: id.into(),
            from_places: from_places.into_iter().map(|s| s.into()).collect(),
            to_place: to_place.into(),
            conditions,
            rules,
        }
    }

    /// Check if this transition can be triggered from the given place
    /// 
    /// This is used by the workflow engine to determine which transitions
    /// are available when a token is in a specific place.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Vector.contains() Method
    /// The `contains()` method checks if a vector contains a specific element.
    /// It returns `bool` and uses the `PartialEq` trait for comparison.
    /// 
    /// ### Reference Parameters
    /// Takes `&PlaceId` to avoid taking ownership - we just want to check
    /// if the place exists in the vector.
    /// 
    /// ### Simple Boolean Logic
    /// Returns `true` if the place is found in from_places, `false` otherwise.
    /// The method is very straightforward but essential for workflow logic.
    pub fn can_trigger_from(&self, place: &PlaceId) -> bool {
        // Check if the given place is in our from_places vector
        // Vec.contains() uses PartialEq to compare elements
        self.from_places.contains(place)
    }
    
    /// Check if all rules pass for the given token
    /// 
    /// This evaluates all structured rules against the token's metadata and data.
    /// All rules must pass for the transition to be enabled.
    /// 
    /// **Note**: This only evaluates structured rules from the `rules` field.
    /// Legacy string-based conditions from the `conditions` field are not evaluated.
    /// Use `RulesEngine::can_transition()` for complete evaluation.
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate rules against
    /// 
    /// ## Returns
    /// `true` if all structured rules pass, `false` if any rule fails
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Iterator all() Method
    /// The `all()` method tests if all elements satisfy a predicate.
    /// It short-circuits - stops as soon as any element returns false.
    pub fn rules_pass(&self, token: &Token) -> bool {
        // All rules must pass for transition to be enabled
        self.rules.iter().all(|rule| rule.evaluate(&token.metadata, &token.data))
    }
    
    /// Check if a token can fire this transition (structured rules only)
    /// 
    /// This combines place compatibility with structured rule evaluation.
    /// Both conditions must be met for the transition to fire.
    /// 
    /// **Important**: This method does NOT evaluate legacy string-based conditions.
    /// For complete evaluation including legacy conditions, use `RulesEngine::can_transition()`.
    /// 
    /// ## Parameters
    /// - `token`: The token attempting to fire the transition
    /// 
    /// ## Returns
    /// `true` if the token can fire this transition based on place and structured rules only
    /// 
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    /// 
    /// // This only evaluates structured rules
    /// // let result = transition.can_fire_with_token(&token);
    /// 
    /// // For complete evaluation including legacy conditions:
    /// let engine = RulesEngine::with_common_rules();
    /// // let can_fire = engine.can_transition(&token, &transition);
    /// // let full_result = engine.evaluate_all_transitions(&token, &workflow);
    /// ```
    pub fn can_fire_with_token(&self, token: &Token) -> bool {
        // Must be in a compatible place AND all structured rules must pass
        self.can_trigger_from(&token.place) && self.rules_pass(token)
    }
    
    /// Get comprehensive evaluation results for debugging
    /// 
    /// This provides detailed information about why a transition can or cannot
    /// fire, including individual rule results and explanations.
    /// 
    /// **Important**: This method only evaluates:
    /// - Place compatibility (is token in the right place?)
    /// - Structured rules (from the `rules` field)
    /// 
    /// **Legacy string-based conditions are NOT evaluated** by this method.
    /// Legacy conditions (from the `conditions` field) are handled by the
    /// `RulesEngine` in the engine layer, which has access to global rule
    /// registries for condition resolution.
    /// 
    /// For complete evaluation including legacy conditions, use:
    /// `RulesEngine::can_transition()` or `RulesEngine::evaluate_all_transitions()`
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate against
    /// 
    /// ## Returns
    /// `TransitionRuleEvaluation` with detailed results for structured rules only
    /// 
    /// ## Use Cases
    /// - Debugging structured rule logic
    /// - Building UIs that show rule-based transition requirements
    /// - Testing rule evaluation in isolation from legacy conditions
    /// 
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    /// 
    /// // This only evaluates structured rules
    /// // let result = transition.evaluate_with_token(&token);
    /// 
    /// // For complete evaluation including legacy conditions:
    /// let engine = RulesEngine::with_common_rules();
    /// // let can_fire = engine.can_transition(&token, &transition);
    /// // let full_result = engine.evaluate_all_transitions(&token, &workflow);
    /// ```
    pub fn evaluate_with_token(&self, token: &Token) -> TransitionRuleEvaluation {
        let place_compatible = self.can_trigger_from(&token.place);
        
        // Evaluate each structured rule individually for detailed feedback
        // NOTE: Legacy string-based conditions (self.conditions) are NOT evaluated here
        // They are handled by the RulesEngine which has access to global rule registries
        let rule_results: Vec<RuleEvaluationResult> = self.rules
            .iter()
            .map(|rule| rule.evaluate_detailed(&token.metadata, &token.data))
            .collect();
        
        let rules_passed = rule_results.iter().all(|result| result.passed);
        let can_fire = place_compatible && rules_passed;
        
        // Generate explanation based on results
        let explanation = if !place_compatible {
            format!(
                "Token is in place '{}' but transition requires one of: {:?}",
                token.place.as_str(),
                self.from_places.iter().map(|p| p.as_str()).collect::<Vec<_>>()
            )
        } else if !rules_passed {
            let failed_count = rule_results.iter().filter(|r| !r.passed).count();
            format!(
                "{} of {} structured rules failed", 
                failed_count, 
                rule_results.len()
            )
        } else {
            "All conditions met - transition can fire (structured rules only)".to_string()
        };
        
        TransitionRuleEvaluation {
            transition_id: self.id.clone(),
            place_compatible,
            rules_passed,
            can_fire,
            rule_results,
            explanation,
        }
    }
    
    /// Add a rule to this transition
    /// 
    /// This allows building up transition rules incrementally.
    /// 
    /// ## Example:
    /// ```
    /// use circuit_breaker::{TransitionDefinition, models::Rule};
    /// use serde_json::json;
    /// 
    /// let mut transition = TransitionDefinition::new("deploy", vec!["tested"], "production");
    /// transition.add_rule(Rule::field_equals("tests", "test_status", json!("passed")));
    /// transition.add_rule(Rule::field_equals("security", "security_status", json!("approved")));
    /// ```
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
    
    /// Remove all rules from this transition
    /// 
    /// This can be useful for testing or dynamic rule modification.
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }
    
    /// Get the number of rules attached to this transition
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    
    /// Check if this transition has any rules
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from the parent module
    use super::super::token::Token; // Import Token for the new test

    #[test]
    fn test_transition_definition() {
        // Test the basic constructor with string literals
        // The Into trait automatically converts &str to PlaceId/TransitionId
        let transition = TransitionDefinition::new(
            "submit",        // &str -> TransitionId
            vec!["draft"],   // Vec<&str> -> Vec<PlaceId>
            "review"         // &str -> PlaceId
        );

        // Test that conversions worked correctly
        assert_eq!(transition.id.as_str(), "submit");
        assert_eq!(transition.from_places.len(), 1);
        assert_eq!(transition.from_places[0].as_str(), "draft");
        assert_eq!(transition.to_place.as_str(), "review");
        
        // Test that conditions start empty
        assert!(transition.conditions.is_empty());
    }

    #[test]
    fn test_transition_with_conditions() {
        // Test the constructor that includes conditions
        let transition = TransitionDefinition::with_conditions(
            "deploy",                   // Transition name
            vec!["tested"],             // Must come from "tested" place
            "production",               // Goes to "production" place
            vec![                       // With these conditions:
                "all_tests_pass".to_string(), 
                "security_approved".to_string()
            ]
        );

        // Verify conditions were stored correctly
        assert_eq!(transition.conditions.len(), 2);
        assert!(transition.conditions.contains(&"all_tests_pass".to_string()));
        assert!(transition.conditions.contains(&"security_approved".to_string()));
    }

    #[test]
    fn test_can_trigger_from() {
        // Test a transition with multiple source places (synchronization)
        let transition = TransitionDefinition::new(
            "approve",                    // Transition name
            vec!["review", "editing"],    // Can trigger from either place
            "approved"                    // Goes to approved
        );

        // Test that it can trigger from both source places
        assert!(transition.can_trigger_from(&PlaceId::from("review")));
        assert!(transition.can_trigger_from(&PlaceId::from("editing")));
        
        // Test that it cannot trigger from other places
        assert!(!transition.can_trigger_from(&PlaceId::from("draft")));
        assert!(!transition.can_trigger_from(&PlaceId::from("published")));
    }
    
    #[test]
    fn test_generic_flexibility() {
        // Test that we can use different types thanks to Into trait bounds
        
        // Using string literals
        let trans1 = TransitionDefinition::new("submit", vec!["draft"], "review");
        
        // Using String objects  
        let id = "submit".to_string();
        let from = "draft".to_string();
        let to = "review".to_string();
        let trans2 = TransitionDefinition::new(id, vec![from], to);
        
        // Using already-constructed types
        let trans3 = TransitionDefinition::new(
            TransitionId::from("submit"),
            vec![PlaceId::from("draft")], 
            PlaceId::from("review")
        );
        
        // All should be equivalent
        assert_eq!(trans1.id.as_str(), trans2.id.as_str());
        assert_eq!(trans2.id.as_str(), trans3.id.as_str());
    }

    #[test]
    fn test_legacy_conditions_not_evaluated() {
        use super::super::rule::Rule;
        
        // Create a transition with both structured rules and legacy conditions
        let mut transition = TransitionDefinition::with_conditions(
            "complex_transition",
            vec!["draft"],
            "review",
            vec!["some_legacy_condition".to_string()] // This won't be evaluated by the transition
        );
        
        // Add a structured rule that will pass
        transition.add_rule(Rule::field_exists("has_content", "content"));
        
        // Create a token that satisfies the structured rule but not the legacy condition
        let mut token = Token::new("test", PlaceId::from("draft"));
        token.data = serde_json::json!({"content": "test content"});
        
        // The transition's own evaluation should pass (ignores legacy conditions)
        let result = transition.evaluate_with_token(&token);
        assert!(result.can_fire); // Passes because structured rule passes
        assert!(result.place_compatible);
        assert!(result.rules_passed);
        assert!(result.explanation.contains("structured rules only"));
        
        // Direct method calls should also pass (structured rules only)
        assert!(transition.can_fire_with_token(&token));
        assert!(transition.rules_pass(&token));
        
        // NOTE: A RulesEngine would be needed to evaluate the legacy condition
        // and might return false if "some_legacy_condition" doesn't resolve to a passing rule
    }
} 