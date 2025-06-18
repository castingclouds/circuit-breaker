// Rules engine for workflow activity evaluation

//! # Rules Engine Module
//!
//! This module provides the central rules engine that evaluates whether resources
//! can execute workflow activities based on their metadata and data.
//!
//! ## Key Features
//!
//! - **Global Rule Registry**: Store and reference rules by name across workflows
//! - **Resource Evaluation**: Determine which activities a resource can execute
//! - **Detailed Feedback**: Provide comprehensive evaluation results for debugging
//! - **Legacy Support**: Maintain compatibility with string-based conditions
//!
//! ## Architecture
//!
//! The rules engine sits between the workflow definition and resource execution:
//! 1. Workflows define activities with rules
//! 2. Rules engine evaluates resource state against activity rules
//! 3. Engine returns available activities and detailed evaluation results
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

use crate::models::{
    activity::ActivityRuleEvaluation, ActivityDefinition, Resource, Rule, RuleCondition,
    WorkflowDefinition,
};
use crate::{CircuitBreakerError, Result};
use async_nats::{
    jetstream::{self, kv},
    Client,
};
use async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Central rules engine for evaluating resource activities
///
/// The `RulesEngine` provides a centralized service for:
/// - Storing global rules that can be referenced by name
/// - Evaluating whether resources can execute specific activities
/// - Finding all available activities for a resource
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
/// // Note: You would need actual resource and workflow instances for full evaluation
/// // let available = engine.available_activities(&resource, &workflow);
/// // let detailed = engine.evaluate_all_activities(&resource, &workflow);
/// ```
///
/// ## Design Philosophy
///
/// The rules engine is **stateless** for thread safety and simplicity.
/// All state is passed in as parameters, making it easy to:
/// - Use in multi-threaded environments
/// - Test with different token/workflow combinations
/// - Cache or persist engine instances
/// Rule storage abstraction for CRUD operations on rules
#[async_trait::async_trait]
pub trait RuleStorage: Send + Sync {
    /// Create a new rule
    async fn create_rule(&self, rule: StoredRule) -> Result<StoredRule>;

    /// Get a rule by ID
    async fn get_rule(&self, id: &str) -> Result<Option<StoredRule>>;

    /// List all rules, optionally filtered by tags
    async fn list_rules(&self, tags: Option<Vec<String>>) -> Result<Vec<StoredRule>>;

    /// Update an existing rule
    async fn update_rule(&self, id: &str, rule: StoredRule) -> Result<StoredRule>;

    /// Delete a rule
    async fn delete_rule(&self, id: &str) -> Result<bool>;

    /// Get rules for a specific workflow
    async fn get_workflow_rules(&self, workflow_id: &str) -> Result<Vec<StoredRule>>;
}

/// A rule with metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub workflow_id: Option<String>, // If rule is workflow-specific
}

impl From<StoredRule> for Rule {
    fn from(stored: StoredRule) -> Self {
        Rule {
            id: stored.id,
            description: stored.description,
            condition: stored.condition,
        }
    }
}

/// NATS KV-based rule storage implementation
pub struct NATSRuleStorage {
    kv_store: kv::Store,
}

impl NATSRuleStorage {
    /// Create a new NATS rule storage
    pub async fn new(nats_client: Client) -> Result<Self> {
        let js = jetstream::new(nats_client);

        // Create or get the rules KV bucket
        let kv_store = js
            .create_key_value(kv::Config {
                bucket: "circuit_breaker_rules".to_string(),
                description: "Circuit Breaker Rules Storage".to_string(),
                max_value_size: 1024 * 1024, // 1MB max rule size
                history: 10,                 // Keep last 10 versions
                ..Default::default()
            })
            .await
            .map_err(|e| CircuitBreakerError::Storage(anyhow::Error::new(e)))?;

        Ok(Self { kv_store })
    }

    /// Get storage key for a rule
    fn rule_key(&self, id: &str) -> String {
        format!("rules.{}", id)
    }

    /// Get storage key for workflow-specific rules
    fn workflow_rules_key(&self, workflow_id: &str) -> String {
        format!("workflow.{}.rules", workflow_id)
    }

    /// Get storage key for rule metadata
    fn rule_metadata_key(&self, id: &str) -> String {
        format!("rules.{}.metadata", id)
    }
}

#[async_trait::async_trait]
impl RuleStorage for NATSRuleStorage {
    async fn create_rule(&self, mut rule: StoredRule) -> Result<StoredRule> {
        rule.created_at = Utc::now();
        rule.updated_at = rule.created_at;

        let rule_json =
            serde_json::to_vec(&rule).map_err(|e| CircuitBreakerError::Serialization(e))?;

        self.kv_store
            .put(self.rule_key(&rule.id), rule_json.into())
            .await
            .map_err(|e| CircuitBreakerError::Storage(anyhow::Error::new(e)))?;

        Ok(rule)
    }

    async fn get_rule(&self, id: &str) -> Result<Option<StoredRule>> {
        match self.kv_store.get(&self.rule_key(id)).await {
            Ok(Some(entry)) => {
                let rule: StoredRule = serde_json::from_slice(&entry)
                    .map_err(|e| CircuitBreakerError::Serialization(e))?;
                Ok(Some(rule))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CircuitBreakerError::Storage(anyhow::Error::new(e))),
        }
    }

    async fn list_rules(&self, tags: Option<Vec<String>>) -> Result<Vec<StoredRule>> {
        let mut keys = self
            .kv_store
            .keys()
            .await
            .map_err(|e| CircuitBreakerError::Storage(anyhow::Error::new(e)))?;

        let mut rules = Vec::new();

        while let Some(key_result) = keys.next().await {
            let key =
                key_result.map_err(|e| CircuitBreakerError::Storage(anyhow::Error::new(e)))?;
            if key.starts_with("rules.") && !key.contains(".metadata") {
                if let Ok(Some(entry)) = self.kv_store.get(&key).await {
                    if let Ok(rule) = serde_json::from_slice::<StoredRule>(&entry) {
                        // Filter by tags if provided
                        if let Some(ref filter_tags) = tags {
                            if filter_tags.iter().any(|tag| rule.tags.contains(tag)) {
                                rules.push(rule);
                            }
                        } else {
                            rules.push(rule);
                        }
                    }
                }
            }
        }

        Ok(rules)
    }

    async fn update_rule(&self, id: &str, mut rule: StoredRule) -> Result<StoredRule> {
        // Get existing rule to preserve created_at
        if let Some(existing) = self.get_rule(id).await? {
            rule.created_at = existing.created_at;
            rule.version = existing.version + 1;
        }

        rule.updated_at = Utc::now();

        let rule_json =
            serde_json::to_vec(&rule).map_err(|e| CircuitBreakerError::Serialization(e))?;

        self.kv_store
            .put(self.rule_key(id), rule_json.into())
            .await
            .map_err(|e| CircuitBreakerError::Storage(anyhow::Error::new(e)))?;

        Ok(rule)
    }

    async fn delete_rule(&self, id: &str) -> Result<bool> {
        match self.kv_store.delete(&self.rule_key(id)).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(false)
                } else {
                    Err(CircuitBreakerError::Storage(anyhow::Error::new(e)))
                }
            }
        }
    }

    async fn get_workflow_rules(&self, workflow_id: &str) -> Result<Vec<StoredRule>> {
        let mut rules = Vec::new();
        let all_rules = self.list_rules(None).await?;

        for rule in all_rules {
            if rule.workflow_id.as_deref() == Some(workflow_id) {
                rules.push(rule);
            }
        }

        Ok(rules)
    }
}

pub struct RulesEngine {
    /// Global rules that can be referenced by name from activities
    ///
    /// This allows defining common rules once and reusing them across
    /// multiple workflows and activities. Rules are stored by their ID.
    global_rules: HashMap<String, Rule>,

    /// Rule storage backend for persistence
    rule_storage: Option<Arc<dyn RuleStorage>>,
}

/// Detailed evaluation results for all activities in a workflow
///
/// This provides comprehensive information about every activity,
/// whether it can execute or not, and why.
#[derive(Debug, Clone)]
pub struct WorkflowEvaluationResult {
    /// ID of the workflow that was evaluated
    pub workflow_id: String,

    /// ID of the resource that was evaluated
    pub resource_id: uuid::Uuid,

    /// Current state of the resource
    pub current_state: String,

    /// Results for each activity in the workflow
    pub activity_results: Vec<ActivityRuleEvaluation>,

    /// Count of activities that can execute
    pub available_count: usize,

    /// Count of activities that cannot execute
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
            rule_storage: None,
        }
    }

    /// Create a new rules engine with NATS storage
    pub fn with_storage(rule_storage: Arc<dyn RuleStorage>) -> Self {
        Self {
            global_rules: HashMap::new(),
            rule_storage: Some(rule_storage),
        }
    }

    /// Get the rule storage backend
    pub fn storage(&self) -> Option<&Arc<dyn RuleStorage>> {
        self.rule_storage.as_ref()
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
    /// // Can now reference "has_content", "status_approved", etc. in activities
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
        engine.register_rule(Rule::field_equals(
            "status_approved",
            "status",
            serde_json::json!("approved"),
        ));
        engine.register_rule(Rule::field_equals(
            "status_rejected",
            "status",
            serde_json::json!("rejected"),
        ));
        engine.register_rule(Rule::field_equals(
            "status_pending",
            "status",
            serde_json::json!("pending"),
        ));

        // Priority and urgency rules
        engine.register_rule(Rule::field_greater_than("high_priority", "priority", 5.0));
        engine.register_rule(Rule::field_greater_than(
            "critical_priority",
            "priority",
            8.0,
        ));
        engine.register_rule(Rule::field_equals(
            "emergency_flag",
            "emergency",
            serde_json::json!(true),
        ));

        // Testing and deployment rules
        engine.register_rule(Rule::field_equals(
            "tests_passed",
            "test_status",
            serde_json::json!("passed"),
        ));
        engine.register_rule(Rule::field_equals(
            "tests_failed",
            "test_status",
            serde_json::json!("failed"),
        ));
        engine.register_rule(Rule::field_equals(
            "security_approved",
            "security_status",
            serde_json::json!("approved"),
        ));
        engine.register_rule(Rule::field_equals(
            "security_flagged",
            "security_status",
            serde_json::json!("flagged"),
        ));

        // User and permission rules
        engine.register_rule(Rule::field_exists("has_assignee", "assignee"));
        engine.register_rule(Rule::field_exists("has_creator", "creator"));
        engine.register_rule(Rule::field_equals(
            "admin_override",
            "admin_override",
            serde_json::json!(true),
        ));

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

    /// Evaluate if a resource can execute a specific activity
    ///
    /// This is the **authoritative method** for complete activity evaluation that combines:
    /// 1. State compatibility (is resource in the right state?)
    /// 2. Structured rule evaluation (do all activity rules pass?)
    /// 3. Legacy condition support (for backwards compatibility)
    ///
    /// **Important**: This is the only method that evaluates legacy string-based conditions.
    /// The `ActivityDefinition::can_execute_with_resource()` method only evaluates structured rules.
    ///
    /// ## Parameters
    /// - `resource`: The resource attempting to execute the activity
    /// - `activity`: The activity definition to evaluate
    ///
    /// ## Returns
    /// `true` if the resource can execute the activity, `false` otherwise
    ///
    /// ## Example
    /// ```rust
    /// use circuit_breaker::RulesEngine;
    ///
    /// let engine = RulesEngine::with_common_rules();
    ///
    /// // This is the complete evaluation including all condition types
    /// // Note: You would need actual resource and activity instances for evaluation
    /// // let can_execute = engine.can_execute_activity(&resource, &activity);
    ///
    /// // Compare with partial evaluation (structured rules only)
    /// // let partial = activity.can_execute_with_resource(&resource);
    /// // can_execute may be false even if partial is true due to legacy conditions
    /// ```
    pub fn can_execute_activity(&self, resource: &Resource, activity: &ActivityDefinition) -> bool {
        // First check state compatibility
        if !activity.can_execute_from(&resource.state) {
            return false;
        }

        // Then evaluate structured rules
        if !activity.rules_pass(resource) {
            return false;
        }

        // Finally evaluate legacy conditions
        self.evaluate_legacy_conditions(resource, activity)
    }

    /// Get all available activities for a resource in a workflow
    ///
    /// This returns all activities that the resource can currently execute.
    /// Useful for building UIs that show available actions.
    ///
    /// ## Parameters
    /// - `resource`: The resource to evaluate
    /// - `workflow`: The workflow definition containing activities
    ///
    /// ## Returns
    /// Vector of references to activities that can execute
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Lifetime Annotations
    /// The return type `Vec<&'a ActivityDefinition>` indicates that
    /// the returned references have the same lifetime as the workflow
    /// parameter. This ensures the references remain valid.
    pub fn available_activities<'a>(
        &self,
        resource: &Resource,
        workflow: &'a WorkflowDefinition,
    ) -> Vec<&'a ActivityDefinition> {
        workflow
            .activities
            .iter()
            .filter(|activity| self.can_execute_activity(resource, activity))
            .collect()
    }

    /// Get detailed evaluation results for all activities
    ///
    /// This provides comprehensive information about every activity
    /// in the workflow, whether it can execute or not, and detailed
    /// explanations of why.
    ///
    /// ## Parameters
    /// - `resource`: The resource to evaluate
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
    pub fn evaluate_all_activities(
        &self,
        resource: &Resource,
        workflow: &WorkflowDefinition,
    ) -> WorkflowEvaluationResult {
        let activity_results: Vec<ActivityRuleEvaluation> = workflow
            .activities
            .iter()
            .map(|activity| {
                let mut result = activity.evaluate_with_resource(resource);

                // Also check legacy conditions and incorporate into result
                if result.can_execute {
                    let legacy_pass = self.evaluate_legacy_conditions(resource, activity);
                    if !legacy_pass {
                        result.can_execute = false;
                        result.explanation =
                            format!("{} (also: legacy conditions failed)", result.explanation);
                    }
                }

                result
            })
            .collect();

        let available_count = activity_results.iter().filter(|r| r.can_execute).count();
        let blocked_count = activity_results.len() - available_count;

        WorkflowEvaluationResult {
            workflow_id: workflow.id.clone(),
            resource_id: resource.id,
            current_state: resource.state.as_str().to_string(),
            activity_results,
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
    /// - `resource`: The resource to evaluate
    /// - `activity`: The activity with legacy conditions
    ///
    /// ## Returns
    /// `true` if all legacy conditions pass, `false` if any fail
    ///
    /// ## Legacy Behavior
    /// - If a condition string matches a global rule ID, evaluate that rule
    /// - If no matching rule found, default to `true` (existing behavior)
    /// - All conditions must pass for the result to be `true`
    fn evaluate_legacy_conditions(
        &self,
        resource: &Resource,
        activity: &ActivityDefinition,
    ) -> bool {
        activity.conditions.iter().all(|condition_name| {
            if let Some(rule) = self.global_rules.get(condition_name) {
                rule.evaluate(&resource.metadata, &resource.data)
            } else {
                // Default to true for unknown conditions (backwards compatibility)
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
    /// - `resource`: The resource to evaluate
    /// - `activity`: The activity with legacy conditions
    ///
    /// ## Returns
    /// Vector of (condition_name, passed, explanation) tuples
    pub fn evaluate_legacy_conditions_detailed(
        &self,
        resource: &Resource,
        activity: &ActivityDefinition,
    ) -> Vec<(String, bool, String)> {
        activity
            .conditions
            .iter()
            .map(|condition_name| {
                if let Some(rule) = self.global_rules.get(condition_name) {
                    let result = rule.evaluate_detailed(&resource.metadata, &resource.data);
                    (condition_name.clone(), result.passed, result.explanation)
                } else {
                    (
                        condition_name.clone(),
                        true,
                        format!(
                            "No rule found for '{}' - defaulting to true",
                            condition_name
                        ),
                    )
                }
            })
            .collect()
    }
}

impl Default for RulesEngine {
    /// Create a default rules engine with common rules pre-loaded
    ///
    /// This is convenient for typical use cases.
    fn default() -> Self {
        Self::with_common_rules()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ActivityDefinition, Resource, StateId, WorkflowDefinition};

    fn create_test_resource() -> Resource {
        let mut resource = Resource::new("test_workflow", StateId::from("draft"));
        resource.set_metadata("reviewer", serde_json::json!("alice@example.com"));
        resource.set_metadata("priority", serde_json::json!(7.5));
        resource.set_metadata("status", serde_json::json!("pending"));
        resource.data = serde_json::json!({"content": "Hello world", "title": "Test Document"});
        resource
    }

    fn create_test_workflow() -> WorkflowDefinition {
        let mut activity = ActivityDefinition::new("submit", vec!["draft"], "review");
        activity.add_rule(Rule::field_exists("has_content", "content"));
        activity.add_rule(Rule::field_exists("has_reviewer", "reviewer"));

        WorkflowDefinition::new(
            "test_workflow",
            "Test Workflow",
            vec![
                StateId::from("draft"),
                StateId::from("review"),
                StateId::from("approved"),
            ],
            vec![
                activity,
                ActivityDefinition::with_rules(
                    "approve",
                    vec!["review"],
                    "approved",
                    vec![Rule::field_equals(
                        "status_approved",
                        "status",
                        serde_json::json!("approved"),
                    )],
                ),
            ],
            "draft",
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
    fn test_activity_evaluation() {
        let engine = RulesEngine::with_common_rules();
        let resource = create_test_resource();
        let workflow = create_test_workflow();

        // Test the submit activity (should pass - has content and reviewer)
        let submit_activity = &workflow.activities[0];
        assert!(engine.can_execute_activity(&resource, submit_activity));

        // Test the approve activity (should fail - status is "pending", not "approved")
        let approve_activity = &workflow.activities[1];
        assert!(!engine.can_execute_activity(&resource, approve_activity));

        // Test available activities
        let available = engine.available_activities(&resource, &workflow);
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id.as_str(), "submit");
    }

    #[test]
    fn test_detailed_evaluation() {
        let engine = RulesEngine::with_common_rules();
        let resource = create_test_resource();
        let workflow = create_test_workflow();

        let result = engine.evaluate_all_activities(&resource, &workflow);

        assert_eq!(result.workflow_id, "test_workflow");
        assert_eq!(result.resource_id, resource.id);
        assert_eq!(result.current_state, "draft");
        assert_eq!(result.activity_results.len(), 2);
        assert_eq!(result.available_count, 1);
        assert_eq!(result.blocked_count, 1);

        // Check specific activity results
        let submit_result = &result.activity_results[0];
        assert!(submit_result.can_execute);
        assert!(submit_result.state_compatible);
        assert!(submit_result.rules_passed);

        let approve_result = &result.activity_results[1];
        assert!(!approve_result.can_execute);
        assert!(!approve_result.state_compatible); // Resource is in "draft", not "review"
    }

    #[test]
    fn test_legacy_conditions() {
        let mut engine = RulesEngine::new();

        // Register a rule that can be referenced by legacy condition
        engine.register_rule(Rule::field_exists("has_content", "content"));

        let resource = create_test_resource();

        // Create activity with legacy string condition
        let activity = ActivityDefinition::with_conditions(
            "submit",
            vec!["draft"],
            "review",
            vec!["has_content".to_string()], // This should resolve to the registered rule
        );

        assert!(engine.can_execute_activity(&resource, &activity));

        // Test detailed legacy evaluation
        let legacy_results = engine.evaluate_legacy_conditions_detailed(&resource, &activity);
        assert_eq!(legacy_results.len(), 1);
        assert_eq!(legacy_results[0].0, "has_content");
        assert!(legacy_results[0].1); // Should pass
    }

    #[test]
    fn test_state_compatibility_check() {
        let engine = RulesEngine::new();
        let mut resource = create_test_resource();

        // Move resource to "review" state
        resource.state = StateId::from("review");

        let activity = ActivityDefinition::new("submit", vec!["draft"], "review");

        // Should fail because resource is in "review" but activity requires "draft"
        assert!(!engine.can_execute_activity(&resource, &activity));
    }

    #[test]
    fn test_complex_rules() {
        let engine = RulesEngine::with_common_rules();
        let resource = create_test_resource();

        // Create activity with complex AND rule
        let complex_rule = Rule::and(
            "complex_approval",
            "Complex approval requirements",
            vec![
                Rule::field_exists("has_content", "content"),
                Rule::field_exists("has_reviewer", "reviewer"),
                Rule::field_greater_than("high_priority", "priority", 5.0),
            ],
        );

        let activity = ActivityDefinition::with_rules(
            "complex_submit",
            vec!["draft"],
            "review",
            vec![complex_rule],
        );

        // Should pass - resource has content, reviewer, and priority > 5
        assert!(engine.can_execute_activity(&resource, &activity));

        // Test detailed evaluation
        let result = activity.evaluate_with_resource(&resource);
        assert!(result.can_execute);
        assert_eq!(result.rule_results.len(), 1);
        assert!(result.rule_results[0].passed);
    }
}
