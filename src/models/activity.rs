// Activity definitions - workflow state change specifications

//! # Activity Definitions
//!
//! This module defines how resources can move between states in a workflow.
//! An `ActivityDefinition` specifies:
//! - Which states a resource can come from (source states)
//! - Which state a resource will go to (target state)
//! - What conditions must be met for the activity to execute
//! - What rules must be satisfied for the activity to be enabled
//!
//! ## Workflow Theory
//!
//! In workflow terminology:
//! - **Input States**: States where resources must be present for activity to execute
//! - **Output State**: State where resources will be moved after activity executes
//! - **Conditions**: Additional business logic that must be satisfied
//! - **Rules**: Structured conditions that can be evaluated against resource state
//!
//! ## Rust Learning Notes:
//!
//! This file demonstrates advanced Rust concepts:
//! - Complex generic functions with multiple type parameters
//! - Trait bounds for flexible APIs
//! - Iterator methods and functional programming
//! - Collection operations (contains, map, collect)

use super::resource::Resource;
use super::rule::{Rule, RuleEvaluationResult}; // Import rules engine
use super::state::{ActivityId, StateId}; // Import from sibling module
use serde::{Deserialize, Serialize}; // JSON serialization traits // Import resource for rule evaluation

/// Generic activity definition
///
/// Defines how resources can move from one or more source states to a target state.
/// This is completely domain-agnostic - any workflow can use this structure.
///
/// ## Examples:
///
/// **Document Review**:
/// - from_states: ["draft"]
/// - to_state: "review"
/// - conditions: ["has_content", "assigned_reviewer"]
/// - rules: [Rule::field_exists("reviewer", "reviewer")]
///
/// **Software Deployment**:
/// - from_states: ["tested"]
/// - to_state: "production"
/// - conditions: ["all_tests_pass", "security_approved"]
/// - rules: [Rule::and("deployment_ready", "Ready for deployment", vec![...])]
///
/// **Order Processing**:
/// - from_states: ["cart"]
/// - to_state: "payment_pending"
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
pub struct ActivityDefinition {
    /// Unique identifier for this activity
    /// Examples: "submit", "approve", "deploy", "cancel"
    pub id: ActivityId,

    /// States where resources must be present for this activity to execute
    /// Can be multiple states for synchronization (join operations)
    /// Examples: ["draft"], ["review", "legal_check"], ["tested", "approved"]
    pub from_states: Vec<StateId>,

    /// State where resources will be moved after activity executes
    /// Examples: "review", "approved", "deployed", "cancelled"
    pub to_state: StateId,

    /// Legacy business logic conditions (kept for backwards compatibility)
    /// These are domain-specific rules checked by the application
    /// Examples: ["has_content"], ["all_tests_pass", "security_scan_clean"]
    pub conditions: Vec<String>, // Generic condition strings

    /// Structured rules that can be evaluated against token state
    /// These provide more sophisticated condition evaluation than simple strings
    /// Rules can check metadata, data fields, and combine with logical operations
    pub rules: Vec<Rule>,
}

/// Results of evaluating structured rules for an activity
///
/// This provides comprehensive information about rule evaluation,
/// useful for debugging and providing user feedback.
///
/// **Important**: This only includes results from structured rules.
/// Legacy string-based conditions are evaluated separately by the RulesEngine.
#[derive(Debug, Clone)]
pub struct ActivityRuleEvaluation {
    /// ID of the activity that was evaluated
    pub activity_id: ActivityId,

    /// Whether the resource is in a compatible state for this activity
    pub state_compatible: bool,

    /// Whether all structured rules passed evaluation
    /// (Legacy conditions are not included in this evaluation)
    pub rules_passed: bool,

    /// Whether the activity can execute based on state and structured rules only
    /// (Legacy conditions are not included - use RulesEngine for complete evaluation)
    pub can_execute: bool,

    /// Detailed results for each structured rule
    pub rule_results: Vec<RuleEvaluationResult>,

    /// Overall explanation of the evaluation
    pub explanation: String,
}

impl ActivityDefinition {
    /// Create a new activity definition
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Complex Generic Function Signature
    /// This function has three generic type parameters with trait bounds:
    /// - `I: Into<ActivityId>` - id can be &str, String, or ActivityId
    /// - `F: Into<StateId>` - each from_state can be &str, String, or StateId
    /// - `T: Into<StateId>` - to_state can be &str, String, or StateId
    ///
    /// ### Trait Bounds (the `where` clause)
    /// The `where` clause makes complex generic signatures more readable.
    /// It's equivalent to writing the bounds inline but cleaner for multiple bounds.
    ///
    /// ### Into<T> Trait
    /// The `Into` trait enables automatic conversions. This means callers can pass:
    /// - String literals: "submit"
    /// - String objects: my_string
    /// - Already-constructed types: ActivityId::from("submit")
    pub fn new<I, F, T>(id: I, from_states: Vec<F>, to_state: T) -> Self
    where
        I: Into<ActivityId>, // id parameter can convert to ActivityId
        F: Into<StateId>,    // each element of from_states can convert to StateId
        T: Into<StateId>,    // to_state parameter can convert to StateId
    {
        ActivityDefinition {
            // Convert the id parameter to ActivityId
            id: id.into(),

            // Convert each element of from_states vector to StateId
            // This uses iterator methods - a functional programming approach
            from_states: from_states
                .into_iter() // Convert vector to iterator (consumes the vector)
                .map(|s| s.into()) // Transform each element using Into trait
                .collect(), // Collect results back into a Vec<StateId>

            // Convert the to_state parameter to StateId
            to_state: to_state.into(),

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
        from_states: Vec<F>,
        to_state: T,
        conditions: Vec<String>, // Conditions are already strings
    ) -> Self
    where
        I: Into<ActivityId>,
        F: Into<StateId>,
        T: Into<StateId>,
    {
        ActivityDefinition {
            id: id.into(),
            // Same iterator pattern as new()
            from_states: from_states.into_iter().map(|s| s.into()).collect(),
            to_state: to_state.into(),
            conditions,    // Move the conditions vector directly
            rules: vec![], // Start with no rules
        }
    }

    /// Create a transition with structured rules
    ///
    /// This is the new preferred way to create activities with sophisticated
    /// condition evaluation using the rules engine.
    ///
    /// ## Example:
    /// ```
    /// use circuit_breaker::{ActivityDefinition, models::Rule};
    /// use serde_json::json;
    ///
    /// let activity = ActivityDefinition::with_rules(
    ///     "deploy",
    ///     vec!["tested"],
    ///     "production",
    ///     vec![
    ///         Rule::field_equals("tests", "test_status", json!("passed")),
    ///         Rule::field_equals("security", "security_status", json!("approved"))
    ///     ]
    /// );
    /// ```
    pub fn with_rules<I, F, T>(id: I, from_states: Vec<F>, to_state: T, rules: Vec<Rule>) -> Self
    where
        I: Into<ActivityId>,
        F: Into<StateId>,
        T: Into<StateId>,
    {
        ActivityDefinition {
            id: id.into(),
            from_states: from_states.into_iter().map(|s| s.into()).collect(),
            to_state: to_state.into(),
            conditions: vec![],
            rules,
        }
    }

    /// Create an activity with both conditions and rules
    ///
    /// This allows for gradual migration from string conditions to structured rules.
    /// Both types of conditions must pass for the activity to execute.
    pub fn with_conditions_and_rules<I, F, T>(
        id: I,
        from_states: Vec<F>,
        to_state: T,
        conditions: Vec<String>,
        rules: Vec<Rule>,
    ) -> Self
    where
        I: Into<ActivityId>,
        F: Into<StateId>,
        T: Into<StateId>,
    {
        ActivityDefinition {
            id: id.into(),
            from_states: from_states.into_iter().map(|s| s.into()).collect(),
            to_state: to_state.into(),
            conditions,
            rules,
        }
    }

    /// Check if this activity can be executed from the given state
    ///
    /// This is used by the workflow engine to determine which activities
    /// are available when a resource is in a specific state.
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Vector.contains() Method
    /// The `contains()` method checks if a vector contains a specific element.
    /// It returns `bool` and uses the `PartialEq` trait for comparison.
    ///
    /// ### Reference Parameters
    /// Takes `&StateId` to avoid taking ownership - we just want to check
    /// if the state exists in the vector.
    ///
    /// ### Simple Boolean Logic
    /// Returns `true` if the state is found in from_states, `false` otherwise.
    /// The method is very straightforward but essential for workflow logic.
    pub fn can_execute_from(&self, state: &StateId) -> bool {
        // Vec.contains() checks if the vector contains the given element
        // Vec.contains() uses PartialEq to compare elements
        self.from_states.contains(state)
    }

    /// Check if all rules pass for the given resource
    ///
    /// This evaluates all structured rules against the resource's metadata and data.
    /// All rules must pass for the activity to be enabled.
    ///
    /// **Note**: This only evaluates structured rules from the `rules` field.
    /// Legacy string-based conditions from the `conditions` field are not evaluated.
    /// Use `RulesEngine::can_execute_activity()` for complete evaluation.
    ///
    /// ## Parameters
    /// - `resource`: The resource to evaluate rules against
    ///
    /// ## Returns
    /// `true` if all structured rules pass, `false` if any rule fails
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Iterator all() Method
    /// The `all()` method tests if all elements satisfy a predicate.
    /// It short-circuits - stops as soon as any element returns false.
    pub fn rules_pass(&self, resource: &Resource) -> bool {
        // All rules must pass for activity to be enabled
        self.rules
            .iter()
            .all(|rule| rule.evaluate(&resource.metadata, &resource.data))
    }

    /// Check if a resource can execute this activity (structured rules only)
    ///
    /// This combines state compatibility with structured rule evaluation.
    /// Both conditions must be met for the activity to execute.
    ///
    /// **Important**: This method does NOT evaluate legacy string-based conditions.
    /// For complete evaluation including legacy conditions, use `RulesEngine::can_execute_activity()`.
    ///
    /// ## Parameters
    /// - `resource`: The resource attempting to execute the activity
    ///
    /// ## Returns
    /// `true` if the resource can execute this activity based on state and structured rules only
    ///
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    ///
    /// // This only evaluates structured rules
    /// // let result = activity.can_execute_with_resource(&resource);
    ///
    /// // For complete evaluation including legacy conditions:
    /// let engine = RulesEngine::with_common_rules();
    /// // let can_execute = engine.can_execute_activity(&resource, &activity);
    /// // let evaluation = engine.evaluate_all_activities(&resource, &activities);
    /// ```
    pub fn can_execute_with_resource(&self, resource: &Resource) -> bool {
        // Must be in a compatible state AND all structured rules must pass
        self.can_execute_from(&resource.state) && self.rules_pass(resource)
    }

    /// Get comprehensive evaluation results for debugging
    ///
    /// This provides detailed information about why an activity can or cannot
    /// execute, including individual rule results and explanations.
    ///
    /// **Important**: This method only evaluates:
    /// - State compatibility (is resource in the right state?)
    /// - Structured rules (from the `rules` field)
    ///
    /// **Legacy string-based conditions are NOT evaluated** by this method.
    /// Legacy conditions (from the `conditions` field) are handled by the
    /// `RulesEngine` in the engine layer, which has access to global rule
    /// registries for condition resolution.
    ///
    /// For complete evaluation including legacy conditions, use:
    /// `RulesEngine::can_execute_activity()` or `RulesEngine::evaluate_all_activities()`
    ///
    /// ## Parameters
    /// - `resource`: The resource to evaluate against
    ///
    /// ## Returns
    /// `ActivityRuleEvaluation` with detailed results for structured rules only
    ///
    /// ## Use Cases
    /// - Debugging structured rule logic
    /// - Building UIs that show rule-based activity requirements
    /// - Testing rule evaluation in isolation from legacy conditions
    ///
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    ///
    /// // This only evaluates structured rules
    /// // let result = activity.evaluate_with_resource(&resource);
    ///
    /// // For complete evaluation including legacy conditions:
    /// let engine = RulesEngine::with_common_rules();
    /// // let can_execute = engine.can_execute_activity(&resource, &activity);
    /// // let evaluation = engine.evaluate_all_activities(&resource, &activities);
    /// ```
    pub fn evaluate_with_resource(&self, resource: &Resource) -> ActivityRuleEvaluation {
        let state_compatible = self.can_execute_from(&resource.state);

        // Evaluate each structured rule individually for detailed feedback
        // NOTE: Legacy string-based conditions (self.conditions) are NOT evaluated here
        // They are handled by the RulesEngine which has access to global rule registries
        let rule_results: Vec<RuleEvaluationResult> = self
            .rules
            .iter()
            .map(|rule| rule.evaluate_detailed(&resource.metadata, &resource.data))
            .collect();

        let rules_passed = rule_results.iter().all(|result| result.passed);
        let can_execute = state_compatible && rules_passed;

        // Generate explanation based on results
        let explanation = if !state_compatible {
            format!(
                "Resource is in state '{}' but activity requires one of: {:?}",
                resource.state.as_str(),
                self.from_states
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
            )
        } else if !rules_passed {
            let failed_count = rule_results.iter().filter(|r| !r.passed).count();
            format!(
                "{} of {} structured rules failed",
                failed_count,
                rule_results.len()
            )
        } else {
            "All conditions met - activity can execute (structured rules only)".to_string()
        };

        ActivityRuleEvaluation {
            activity_id: self.id.clone(),
            state_compatible,
            rules_passed,
            can_execute,
            rule_results,
            explanation,
        }
    }

    /// Add a rule to this activity
    ///
    /// This allows building up activity rules incrementally.
    ///
    /// ## Example:
    /// ```
    /// use circuit_breaker::{ActivityDefinition, models::Rule};
    /// use serde_json::json;
    ///
    /// let mut activity = ActivityDefinition::new("deploy", vec!["tested"], "production");
    /// activity.add_rule(Rule::field_equals("tests", "test_status", json!("passed")));
    /// activity.add_rule(Rule::field_equals("security", "security_status", json!("approved")));
    /// ```
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Remove all rules from this activity
    ///
    /// This can be useful for testing or dynamic rule modification.
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Get the number of rules attached to this activity
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Check if this activity has any rules
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::resource::Resource;
    use super::*; // Import everything from the parent module

    #[test]
    fn test_activity_definition() {
        // Test the basic constructor with string literals
        // The Into trait automatically converts &str to StateId/ActivityId
        let activity = ActivityDefinition::new(
            "submit",      // &str -> ActivityId
            vec!["draft"], // Vec<&str> -> Vec<StateId>
            "review",      // &str -> StateId
        );

        // Test that conversions worked correctly
        assert_eq!(activity.id.as_str(), "submit");
        assert_eq!(activity.from_states.len(), 1);
        assert_eq!(activity.from_states[0].as_str(), "draft");
        assert_eq!(activity.to_state.as_str(), "review");

        // Test that conditions start empty
        assert!(activity.conditions.is_empty());
    }

    #[test]
    fn test_activity_with_conditions() {
        // Test the constructor that includes conditions
        let activity = ActivityDefinition::with_conditions(
            "deploy",       // Activity name
            vec!["tested"], // Must come from "tested" state
            "production",   // Goes to "production" state
            vec![
                // With these conditions:
                "all_tests_pass".to_string(),
                "security_approved".to_string(),
            ],
        );

        // Verify conditions were stored correctly
        assert_eq!(activity.conditions.len(), 2);
        assert!(activity.conditions.contains(&"all_tests_pass".to_string()));
        assert!(activity
            .conditions
            .contains(&"security_approved".to_string()));
    }

    #[test]
    fn test_can_execute_from() {
        // Test an activity with multiple source states (synchronization)
        let activity = ActivityDefinition::new(
            "approve",                 // Activity name
            vec!["review", "editing"], // Can execute from either state
            "approved",                // Goes to approved
        );

        // Test that it can execute from both source states
        assert!(activity.can_execute_from(&StateId::from("review")));
        assert!(activity.can_execute_from(&StateId::from("editing")));

        // Test that it cannot execute from other states
        assert!(!activity.can_execute_from(&StateId::from("draft")));
        assert!(!activity.can_execute_from(&StateId::from("published")));
    }

    #[test]
    fn test_generic_flexibility() {
        // Test that we can use different types thanks to Into trait bounds

        // Using string literals
        let act1 = ActivityDefinition::new("submit", vec!["draft"], "review");

        // Using String objects
        let id = "submit".to_string();
        let from = "draft".to_string();
        let to = "review".to_string();
        let act2 = ActivityDefinition::new(id, vec![from], to);

        // Using already-constructed types
        let act3 = ActivityDefinition::new(
            ActivityId::from("submit"),
            vec![StateId::from("draft")],
            StateId::from("review"),
        );

        // All should be equivalent
        assert_eq!(act1.id.as_str(), act2.id.as_str());
        assert_eq!(act2.id.as_str(), act3.id.as_str());
    }

    #[test]
    fn test_legacy_conditions_not_evaluated() {
        use super::super::rule::Rule;

        // Create an activity with both structured rules and legacy conditions
        let mut activity = ActivityDefinition::with_conditions(
            "complex_activity",
            vec!["draft"],
            "review",
            vec!["some_legacy_condition".to_string()], // This won't be evaluated by the activity
        );

        // Add a structured rule that will pass
        activity.add_rule(Rule::field_exists("has_content", "content"));

        // Create a resource that satisfies the structured rule but not the legacy condition
        let mut resource = Resource::new("test", StateId::from("draft"));
        resource.data = serde_json::json!({"content": "test content"});

        // The activity's own evaluation should pass (ignores legacy conditions)
        let result = activity.evaluate_with_resource(&resource);
        assert!(result.can_execute); // Passes because structured rule passes
        assert!(result.state_compatible);
        assert!(result.rules_passed);
        assert!(result.explanation.contains("structured rules only"));

        // Direct method calls should also pass (structured rules only)
        assert!(activity.can_execute_with_resource(&resource));
        assert!(activity.rules_pass(&resource));

        // NOTE: A RulesEngine would be needed to evaluate the legacy condition
        // and might return false if "some_legacy_condition" doesn't resolve to a passing rule
    }
}
