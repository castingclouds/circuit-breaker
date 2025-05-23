// Rules engine for workflow transition evaluation

//! # Rules Engine Module
//! 
//! This module provides the central rules engine that evaluates whether tokens
//! can transition through workflow transitions based on their metadata and data.
//! 
//! ## Key Features
//! 
//! - **Global Rule Registry**: Store and reference rules by name across workflows
//! - **Token Evaluation**: Determine which transitions a token can fire
//! - **Detailed Feedback**: Provide comprehensive evaluation results for debugging
//! - **Legacy Support**: Maintain compatibility with string-based conditions
//! 
//! ## Architecture
//! 
//! The rules engine sits between the workflow definition and token execution:
//! 1. Workflows define transitions with rules
//! 2. Rules engine evaluates token state against transition rules
//! 3. Engine returns available transitions and detailed evaluation results
//! 
//! ## Rust Learning Notes:
//! 
//! ### HashMap for Fast Lookups
//! We use `HashMap<String, Rule>` to store global rules for O(1) lookup
//! by rule name. This is much faster than scanning through vectors.
//! 
//! ### Lifetime Parameters
//! Some methods return references to workflow data, requiring lifetime
//! annotations to ensure the references remain valid.

use std::collections::HashMap;
use crate::models::{
    Token, WorkflowDefinition, TransitionDefinition, Rule,
    transition::TransitionRuleEvaluation
};

/// Central rules engine for evaluating token transitions
/// 
/// The `RulesEngine` provides a centralized service for:
/// - Storing global rules that can be referenced by name
/// - Evaluating whether tokens can fire specific transitions
/// - Finding all available transitions for a token
/// - Providing detailed evaluation feedback for debugging
/// 
/// ## Usage Example:
/// 
/// ```rust
/// use circuit_breaker::{RulesEngine, models::Rule};
/// use serde_json::json;
/// 
/// let mut engine = RulesEngine::new();
/// 
/// // Register global rules
/// engine.register_rule(Rule::field_exists("has_content", "content"));
/// engine.register_rule(Rule::field_equals("approved", "status", json!("approved")));
/// 
/// // Note: You would need actual token and workflow instances for full evaluation
/// // let available = engine.available_transitions(&token, &workflow);
/// // let detailed = engine.evaluate_all_transitions(&token, &workflow);
/// ```
/// 
/// ## Design Philosophy
/// 
/// The rules engine is **stateless** for thread safety and simplicity.
/// All state is passed in as parameters, making it easy to:
/// - Use in multi-threaded environments
/// - Test with different token/workflow combinations
/// - Cache or persist engine instances
pub struct RulesEngine {
    /// Global rules that can be referenced by name from transitions
    /// 
    /// This allows defining common rules once and reusing them across
    /// multiple workflows and transitions. Rules are stored by their ID.
    global_rules: HashMap<String, Rule>,
}

/// Detailed evaluation results for all transitions in a workflow
/// 
/// This provides comprehensive information about every transition,
/// whether it can fire or not, and why.
#[derive(Debug, Clone)]
pub struct WorkflowEvaluationResult {
    /// ID of the workflow that was evaluated
    pub workflow_id: String,
    
    /// ID of the token that was evaluated
    pub token_id: uuid::Uuid,
    
    /// Current place of the token
    pub current_place: String,
    
    /// Results for each transition in the workflow
    pub transition_results: Vec<TransitionRuleEvaluation>,
    
    /// Count of transitions that can fire
    pub available_count: usize,
    
    /// Count of transitions that cannot fire
    pub blocked_count: usize,
}

impl RulesEngine {
    /// Create a new rules engine
    /// 
    /// The engine starts empty - rules must be registered before use.
    /// 
    /// ## Example:
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    /// 
    /// let engine = RulesEngine::new();
    /// ```
    pub fn new() -> Self {
        Self {
            global_rules: HashMap::new(),
        }
    }
    
    /// Create a rules engine with common predefined rules
    /// 
    /// This provides a starting set of useful rules that cover common
    /// workflow scenarios like approvals, content validation, etc.
    /// 
    /// ## Predefined Rules:
    /// - `has_content`: Checks if "content" field exists
    /// - `has_reviewer`: Checks if "reviewer" field exists  
    /// - `status_approved`: Checks if status equals "approved"
    /// - `status_rejected`: Checks if status equals "rejected"
    /// - `high_priority`: Checks if priority > 5
    /// - `emergency_flag`: Checks if emergency field is true
    /// 
    /// ## Example:
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    /// 
    /// let engine = RulesEngine::with_common_rules();
    /// // Can now reference "has_content", "status_approved", etc. in transitions
    /// ```
    pub fn with_common_rules() -> Self {
        let mut engine = Self::new();
        
        // Content validation rules
        engine.register_rule(Rule::field_exists("has_content", "content"));
        engine.register_rule(Rule::field_exists("has_title", "title"));
        engine.register_rule(Rule::field_exists("has_description", "description"));
        
        // Approval workflow rules
        engine.register_rule(Rule::field_exists("has_reviewer", "reviewer"));
        engine.register_rule(Rule::field_exists("has_approver", "approver"));
        engine.register_rule(Rule::field_equals("status_approved", "status", serde_json::json!("approved")));
        engine.register_rule(Rule::field_equals("status_rejected", "status", serde_json::json!("rejected")));
        engine.register_rule(Rule::field_equals("status_pending", "status", serde_json::json!("pending")));
        
        // Priority and urgency rules
        engine.register_rule(Rule::field_greater_than("high_priority", "priority", 5.0));
        engine.register_rule(Rule::field_greater_than("critical_priority", "priority", 8.0));
        engine.register_rule(Rule::field_equals("emergency_flag", "emergency", serde_json::json!(true)));
        
        // Testing and deployment rules
        engine.register_rule(Rule::field_equals("tests_passed", "test_status", serde_json::json!("passed")));
        engine.register_rule(Rule::field_equals("tests_failed", "test_status", serde_json::json!("failed")));
        engine.register_rule(Rule::field_equals("security_approved", "security_status", serde_json::json!("approved")));
        engine.register_rule(Rule::field_equals("security_flagged", "security_status", serde_json::json!("flagged")));
        
        // User and permission rules
        engine.register_rule(Rule::field_exists("has_assignee", "assignee"));
        engine.register_rule(Rule::field_exists("has_creator", "creator"));
        engine.register_rule(Rule::field_equals("admin_override", "admin_override", serde_json::json!(true)));
        
        engine
    }
    
    /// Register a global rule that can be referenced by name
    /// 
    /// Global rules can be reused across multiple workflows and transitions.
    /// If a rule with the same ID already exists, it will be replaced.
    /// 
    /// ## Parameters
    /// - `rule`: The rule to register
    /// 
    /// ## Example:
    /// ```rust
    /// use circuit_breaker::{RulesEngine, models::Rule};
    /// use serde_json::json;
    /// 
    /// let mut engine = RulesEngine::new();
    /// engine.register_rule(Rule::field_equals("custom_status", "status", json!("custom")));
    /// ```
    pub fn register_rule(&mut self, rule: Rule) {
        self.global_rules.insert(rule.id.clone(), rule);
    }
    
    /// Get a global rule by ID
    /// 
    /// Returns a reference to the rule if found, or None if not registered.
    /// 
    /// ## Parameters
    /// - `rule_id`: The ID of the rule to retrieve
    /// 
    /// ## Returns
    /// `Some(&Rule)` if found, `None` if not registered
    pub fn get_rule(&self, rule_id: &str) -> Option<&Rule> {
        self.global_rules.get(rule_id)
    }
    
    /// Get all registered rule IDs
    /// 
    /// Useful for debugging or building UIs that show available rules.
    /// 
    /// ## Returns
    /// Vector of all registered rule IDs
    pub fn list_rule_ids(&self) -> Vec<String> {
        self.global_rules.keys().cloned().collect()
    }
    
    /// Remove a global rule
    /// 
    /// ## Parameters
    /// - `rule_id`: The ID of the rule to remove
    /// 
    /// ## Returns
    /// `Some(Rule)` if the rule was found and removed, `None` if not found
    pub fn remove_rule(&mut self, rule_id: &str) -> Option<Rule> {
        self.global_rules.remove(rule_id)
    }
    
    /// Clear all global rules
    /// 
    /// This removes all registered rules. Useful for testing or resetting state.
    pub fn clear_rules(&mut self) {
        self.global_rules.clear();
    }
    
    /// Evaluate if a token can fire a specific transition
    /// 
    /// This is the **authoritative method** for complete transition evaluation that combines:
    /// 1. Place compatibility (is token in the right place?)
    /// 2. Structured rule evaluation (do all transition rules pass?)
    /// 3. Legacy condition support (for backwards compatibility)
    /// 
    /// **Important**: This is the only method that evaluates legacy string-based conditions.
    /// The `TransitionDefinition::can_fire_with_token()` method only evaluates structured rules.
    /// 
    /// ## Parameters
    /// - `token`: The token attempting to fire the transition
    /// - `transition`: The transition definition to evaluate
    /// 
    /// ## Returns
    /// `true` if the token can fire the transition, `false` otherwise
    /// 
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    /// 
    /// let engine = RulesEngine::with_common_rules();
    /// 
    /// // This is the complete evaluation including all condition types
    /// // Note: You would need actual token and transition instances for evaluation
    /// // let can_fire = engine.can_transition(&token, &transition);
    /// 
    /// // Compare with partial evaluation (structured rules only)
    /// // let partial = transition.can_fire_with_token(&token);
    /// // can_fire may be false even if partial is true due to legacy conditions
    /// ```
    pub fn can_transition(&self, token: &Token, transition: &TransitionDefinition) -> bool {
        // First check place compatibility (cheap check)
        if !transition.can_trigger_from(&token.place) {
            return false;
        }
        
        // Then evaluate structured rules
        if !transition.rules_pass(token) {
            return false;
        }
        
        // Finally evaluate legacy conditions
        self.evaluate_legacy_conditions(token, transition)
    }
    
    /// Get all available transitions for a token in a workflow
    /// 
    /// This returns all transitions that the token can currently fire.
    /// Useful for building UIs that show available actions.
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate
    /// - `workflow`: The workflow definition containing transitions
    /// 
    /// ## Returns
    /// Vector of references to transitions that can fire
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Lifetime Annotations
    /// The return type `Vec<&'a TransitionDefinition>` indicates that
    /// the returned references have the same lifetime as the workflow
    /// parameter. This ensures the references remain valid.
    pub fn available_transitions<'a>(
        &self, 
        token: &Token, 
        workflow: &'a WorkflowDefinition
    ) -> Vec<&'a TransitionDefinition> {
        workflow.transitions
            .iter()
            .filter(|transition| self.can_transition(token, transition))
            .collect()
    }
    
    /// Get detailed evaluation results for all transitions
    /// 
    /// This provides comprehensive information about every transition
    /// in the workflow, whether it can fire or not, and detailed
    /// explanations of why.
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate
    /// - `workflow`: The workflow definition
    /// 
    /// ## Returns
    /// `WorkflowEvaluationResult` with detailed information
    /// 
    /// ## Use Cases
    /// - Debugging workflow logic
    /// - Building administrative UIs
    /// - Providing detailed user feedback
    /// - Workflow optimization and analysis
    pub fn evaluate_all_transitions(&self, token: &Token, workflow: &WorkflowDefinition) -> WorkflowEvaluationResult {
        let transition_results: Vec<TransitionRuleEvaluation> = workflow.transitions
            .iter()
            .map(|transition| {
                let mut result = transition.evaluate_with_token(token);
                
                // Also check legacy conditions and incorporate into result
                if result.can_fire {
                    let legacy_pass = self.evaluate_legacy_conditions(token, transition);
                    if !legacy_pass {
                        result.can_fire = false;
                        result.explanation = format!("{} (also: legacy conditions failed)", result.explanation);
                    }
                }
                
                result
            })
            .collect();
            
        let available_count = transition_results.iter().filter(|r| r.can_fire).count();
        let blocked_count = transition_results.len() - available_count;
        
        WorkflowEvaluationResult {
            workflow_id: workflow.id.clone(),
            token_id: token.id,
            current_place: token.current_place().to_string(),
            transition_results,
            available_count,
            blocked_count,
        }
    }
    
    /// Evaluate legacy string-based conditions
    /// 
    /// This provides backwards compatibility with the original string-based
    /// condition system. String conditions are looked up as global rule names.
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate
    /// - `transition`: The transition with legacy conditions
    /// 
    /// ## Returns
    /// `true` if all legacy conditions pass, `false` if any fail
    /// 
    /// ## Legacy Behavior
    /// - If a condition string matches a global rule ID, evaluate that rule
    /// - If no matching rule found, default to `true` (existing behavior)
    /// - Empty conditions list evaluates to `true`
    fn evaluate_legacy_conditions(&self, token: &Token, transition: &TransitionDefinition) -> bool {
        transition.conditions.iter().all(|condition_name| {
            if let Some(rule) = self.global_rules.get(condition_name) {
                // Found a matching global rule - evaluate it
                rule.evaluate(&token.metadata, &token.data)
            } else {
                // No matching rule found - default to true for backwards compatibility
                true
            }
        })
    }
    
    /// Get detailed legacy condition evaluation results
    /// 
    /// This provides detailed feedback about legacy condition evaluation,
    /// useful for migration from string conditions to structured rules.
    /// 
    /// ## Parameters
    /// - `token`: The token to evaluate
    /// - `transition`: The transition with legacy conditions
    /// 
    /// ## Returns
    /// Vector of (condition_name, passed, explanation) tuples
    pub fn evaluate_legacy_conditions_detailed(&self, token: &Token, transition: &TransitionDefinition) -> Vec<(String, bool, String)> {
        transition.conditions.iter().map(|condition_name| {
            if let Some(rule) = self.global_rules.get(condition_name) {
                let result = rule.evaluate_detailed(&token.metadata, &token.data);
                (condition_name.clone(), result.passed, result.explanation)
            } else {
                (
                    condition_name.clone(), 
                    true, 
                    format!("No rule found for '{}' - defaulting to true", condition_name)
                )
            }
        }).collect()
    }
}

impl Default for RulesEngine {
    /// Create a default rules engine with common rules pre-loaded
    /// 
    /// This is equivalent to calling `RulesEngine::with_common_rules()`.
    fn default() -> Self {
        Self::with_common_rules()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Token, WorkflowDefinition, TransitionDefinition, PlaceId};

    fn create_test_token() -> Token {
        let mut token = Token::new("test_workflow", PlaceId::from("draft"));
        token.set_metadata("status", serde_json::json!("pending"));
        token.set_metadata("priority", serde_json::json!(7));
        token.set_metadata("reviewer", serde_json::json!("alice"));
        token.data = serde_json::json!({"content": "Hello world", "title": "Test Document"});
        token
    }
    
    fn create_test_workflow() -> WorkflowDefinition {
        let mut transition = TransitionDefinition::new("submit", vec!["draft"], "review");
        transition.add_rule(Rule::field_exists("has_content", "content"));
        transition.add_rule(Rule::field_exists("has_reviewer", "reviewer"));
        
        WorkflowDefinition::new(
            "test_workflow",
            "Test Workflow",
            vec![PlaceId::from("draft"), PlaceId::from("review"), PlaceId::from("approved")],
            vec![
                transition,
                TransitionDefinition::with_rules(
                    "approve",
                    vec!["review"],
                    "approved",
                    vec![Rule::field_equals("status_approved", "status", serde_json::json!("approved"))]
                )
            ],
            "draft"
        )
    }

    #[test]
    fn test_rules_engine_creation() {
        let engine = RulesEngine::new();
        assert_eq!(engine.list_rule_ids().len(), 0);
        
        let engine_with_rules = RulesEngine::with_common_rules();
        assert!(engine_with_rules.list_rule_ids().len() > 0);
        assert!(engine_with_rules.get_rule("has_content").is_some());
        assert!(engine_with_rules.get_rule("status_approved").is_some());
    }
    
    #[test]
    fn test_rule_registration() {
        let mut engine = RulesEngine::new();
        
        let rule = Rule::field_exists("custom_rule", "custom_field");
        engine.register_rule(rule.clone());
        
        assert_eq!(engine.list_rule_ids().len(), 1);
        assert!(engine.get_rule("custom_rule").is_some());
        assert_eq!(engine.get_rule("custom_rule").unwrap().id, "custom_rule");
        
        // Test removal
        let removed = engine.remove_rule("custom_rule");
        assert!(removed.is_some());
        assert_eq!(engine.list_rule_ids().len(), 0);
    }
    
    #[test]
    fn test_transition_evaluation() {
        let engine = RulesEngine::with_common_rules();
        let token = create_test_token();
        let workflow = create_test_workflow();
        
        // Test the submit transition (should pass - has content and reviewer)
        let submit_transition = &workflow.transitions[0];
        assert!(engine.can_transition(&token, submit_transition));
        
        // Test the approve transition (should fail - status is "pending", not "approved")
        let approve_transition = &workflow.transitions[1];
        assert!(!engine.can_transition(&token, approve_transition));
        
        // Test available transitions
        let available = engine.available_transitions(&token, &workflow);
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id.as_str(), "submit");
    }
    
    #[test]
    fn test_detailed_evaluation() {
        let engine = RulesEngine::with_common_rules();
        let token = create_test_token();
        let workflow = create_test_workflow();
        
        let result = engine.evaluate_all_transitions(&token, &workflow);
        
        assert_eq!(result.workflow_id, "test_workflow");
        assert_eq!(result.token_id, token.id);
        assert_eq!(result.current_place, "draft");
        assert_eq!(result.transition_results.len(), 2);
        assert_eq!(result.available_count, 1);
        assert_eq!(result.blocked_count, 1);
        
        // Check specific transition results
        let submit_result = &result.transition_results[0];
        assert!(submit_result.can_fire);
        assert!(submit_result.place_compatible);
        assert!(submit_result.rules_passed);
        
        let approve_result = &result.transition_results[1];
        assert!(!approve_result.can_fire);
        assert!(!approve_result.place_compatible); // Token is in "draft", not "review"
    }
    
    #[test]
    fn test_legacy_conditions() {
        let mut engine = RulesEngine::new();
        
        // Register a rule that can be referenced by legacy condition
        engine.register_rule(Rule::field_exists("has_content", "content"));
        
        let token = create_test_token();
        
        // Create transition with legacy string condition
        let transition = TransitionDefinition::with_conditions(
            "submit",
            vec!["draft"],
            "review",
            vec!["has_content".to_string()] // This should resolve to the registered rule
        );
        
        assert!(engine.can_transition(&token, &transition));
        
        // Test detailed legacy evaluation
        let legacy_results = engine.evaluate_legacy_conditions_detailed(&token, &transition);
        assert_eq!(legacy_results.len(), 1);
        assert_eq!(legacy_results[0].0, "has_content");
        assert!(legacy_results[0].1); // Should pass
    }
    
    #[test]
    fn test_place_compatibility_check() {
        let engine = RulesEngine::new();
        let mut token = create_test_token();
        
        // Move token to "review" place
        token.place = PlaceId::from("review");
        
        let transition = TransitionDefinition::new("submit", vec!["draft"], "review");
        
        // Should fail because token is in "review" but transition requires "draft"
        assert!(!engine.can_transition(&token, &transition));
    }
    
    #[test]
    fn test_complex_rules() {
        let engine = RulesEngine::with_common_rules();
        let token = create_test_token();
        
        // Create transition with complex AND rule
        let complex_rule = Rule::and(
            "complex_approval",
            "Complex approval requirements",
            vec![
                Rule::field_exists("has_content", "content"),
                Rule::field_exists("has_reviewer", "reviewer"),
                Rule::field_greater_than("high_priority", "priority", 5.0),
            ]
        );
        
        let transition = TransitionDefinition::with_rules(
            "complex_submit",
            vec!["draft"],
            "review",
            vec![complex_rule]
        );
        
        // Should pass - token has content, reviewer, and priority > 5
        assert!(engine.can_transition(&token, &transition));
        
        // Test detailed evaluation
        let result = transition.evaluate_with_token(&token);
        assert!(result.can_fire);
        assert_eq!(result.rule_results.len(), 1);
        assert!(result.rule_results[0].passed);
    }
} 