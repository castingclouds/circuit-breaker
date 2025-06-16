// Workflow States and Activities - intuitive workflow foundations
//
// ## Workflow Model Overview
//
// Circuit Breaker implements **State Managed Workflows** using an intuitive model:
// **Resources flow through States via Activities**
//
// ### Core Concepts:
//
// **States (StateId)**: Where resources currently are in the workflow.
// - Example: "draft", "review", "approved", "rejected"
// - Resources indicate the current state of workflow execution
// - Multiple resources can exist in different states simultaneously
//
// **Activities (ActivityId)**: Actions that move resources between states.
// - Example: "submit", "approve", "reject", "revise"
// - Can have multiple input states (synchronization)
// - Can have multiple output states (branching)
// - Enable cycles (impossible in DAGs)
//
// **Resources**: The things being worked on in the workflow.
// - Each resource carries domain-specific data
// - Resources move from state to state via activities
// - History of all movements is preserved for audit trails
//
// ### State Management Advantages over DAGs:
//
// 1. **Cycles Supported**: Revision loops, retries, rollbacks are natural
// 2. **Concurrent Execution**: Multiple resources in different states
// 3. **Synchronization**: Activities can wait for multiple input resources
// 4. **Mathematical Verification**: Formal analysis of deadlocks, liveness
// 5. **Flexible Branching**: Complex decision points with multiple outcomes
//
// ### Example Workflow:
//
// ```
//     [draft] --submit--> [review] --approve--> [published]
//        ^                   |
//        |                   |reject
//        +---<--revise--<----+
// ```
//
// This supports revision cycles that are impossible to represent in DAGs.
//
// ### Implementation Notes:
//
// - StateId and ActivityId are simple string wrappers for maximum flexibility
// - Any domain can define their own state and activity names
// - The engine is completely generic - no hardcoded workflow knowledge
// - GraphQL clients define domain-specific workflows at runtime

use serde::{Deserialize, Serialize};

/// **Workflow State** - represents where resources currently are
///
/// A **state** is a location where resources exist in the workflow.
/// States represent conditions, statuses, or phases in the modeled system.
///
/// ## Examples by Domain:
///
/// **Document Workflow**: "draft", "review", "approved", "rejected"
/// **Software Deployment**: "development", "staging", "production", "rollback"
/// **Order Processing**: "cart", "payment", "fulfillment", "shipped", "delivered"
/// **AI Agent Campaign**: "planning", "research", "generation", "review", "published"
///
/// ## Design Principles:
///
/// - **Generic**: Any string can be a state name
/// - **Domain-Agnostic**: No hardcoded business logic
/// - **Flexible**: Client applications define their own state semantics
/// - **Immutable**: State identities don't change once created
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

impl StateId {
    /// Get the state identifier as a string slice
    ///
    /// ```rust
    /// # use circuit_breaker::StateId;
    /// let state = StateId::from("draft");
    /// assert_eq!(state.as_str(), "draft");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a new state from any string-like input
    ///
    /// ```rust
    /// # use circuit_breaker::StateId;
    /// let state1 = StateId::new("review");
    /// let state2 = StateId::from("review");
    /// assert_eq!(state1, state2);
    /// ```
    pub fn new<S: Into<String>>(name: S) -> Self {
        StateId(name.into())
    }
}

impl From<&str> for StateId {
    fn from(s: &str) -> Self {
        StateId(s.to_string())
    }
}

impl From<String> for StateId {
    fn from(s: String) -> Self {
        StateId(s)
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// **Workflow Activity** - represents an action that moves resources between states
///
/// An **activity** is an active component that moves resources
/// from input states to output states. Activities represent
/// actions, events, or computations in the modeled system.
///
/// ## Examples by Domain:
///
/// **Document Workflow**: "submit", "approve", "reject", "revise", "publish"
/// **Software Deployment**: "build", "test", "deploy", "rollback", "scale"
/// **Order Processing**: "add_to_cart", "checkout", "pay", "ship", "deliver"
/// **AI Agent Campaign**: "research", "generate", "review", "optimize", "publish"
///
/// ## Activity Execution Rules:
///
/// 1. **Preconditions**: All required input states must contain resources
/// 2. **Action**: Activity executes, moving input resources
/// 3. **Postconditions**: Resources are moved to output states
/// 4. **Atomicity**: Either all changes happen or none (transaction semantics)
///
/// ## Advanced Patterns:
///
/// - **Synchronization**: Activity waits for resources in multiple input states
/// - **Branching**: Activity moves resources to multiple output states
/// - **Choice**: Multiple activities compete for the same input resources
/// - **Cycles**: Activities can create loops (revision cycles, retries)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActivityId(pub String);

impl ActivityId {
    /// Get the activity identifier as a string slice
    ///
    /// ```rust
    /// # use circuit_breaker::ActivityId;
    /// let activity = ActivityId::from("submit");
    /// assert_eq!(activity.as_str(), "submit");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a new activity from any string-like input
    ///
    /// ```rust
    /// # use circuit_breaker::ActivityId;
    /// let act1 = ActivityId::new("approve");
    /// let act2 = ActivityId::from("approve");
    /// assert_eq!(act1, act2);
    /// ```
    pub fn new<S: Into<String>>(name: S) -> Self {
        ActivityId(name.into())
    }
}

impl From<&str> for ActivityId {
    fn from(s: &str) -> Self {
        ActivityId(s.to_string())
    }
}

impl From<String> for ActivityId {
    fn from(s: String) -> Self {
        ActivityId(s)
    }
}

impl std::fmt::Display for ActivityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_id_creation() {
        let state1 = StateId::from("draft");
        let state2 = StateId::from("draft".to_string());
        let state3 = StateId("draft".to_string());
        let state4 = StateId::new("draft");

        assert_eq!(state1, state2);
        assert_eq!(state2, state3);
        assert_eq!(state3, state4);
        assert_eq!(state1.as_str(), "draft");
        assert_eq!(state1.to_string(), "draft");
    }

    #[test]
    fn test_activity_id_creation() {
        let act1 = ActivityId::from("submit");
        let act2 = ActivityId::from("submit".to_string());
        let act3 = ActivityId::new("submit");

        assert_eq!(act1, act2);
        assert_eq!(act2, act3);
        assert_eq!(act1.as_str(), "submit");
        assert_eq!(act1.to_string(), "submit");
    }

    #[test]
    fn test_workflow_example() {
        // Model a simple document review process
        let draft = StateId::from("draft");
        let review = StateId::from("review");
        let _approved = StateId::from("approved");
        let _rejected = StateId::from("rejected");

        let submit = ActivityId::from("submit");
        let approve = ActivityId::from("approve");
        let _reject = ActivityId::from("reject");
        let _revise = ActivityId::from("revise");

        // This workflow supports cycles: draft -> review -> rejected -> draft
        // Which is impossible to model with DAGs!
        assert_ne!(draft, review);
        assert_ne!(submit, approve);

        // States and activities have meaningful string representations
        assert_eq!(format!("{}", draft), "draft");
        assert_eq!(format!("{}", submit), "submit");
    }

    #[test]
    fn test_generic_workflow_modeling() {
        // Demonstrate that any domain can be modeled

        // E-commerce workflow
        let cart = StateId::from("cart");
        let _checkout = StateId::from("checkout");
        let _paid = StateId::from("paid");
        let add_item = ActivityId::from("add_item");
        let _purchase = ActivityId::from("purchase");

        // AI agent workflow
        let planning = StateId::from("planning");
        let executing = StateId::from("executing");
        let _complete = StateId::from("complete");
        let start_task = ActivityId::from("start_task");
        let _finish_task = ActivityId::from("finish_task");

        // Software deployment workflow
        let development = StateId::from("development");
        let _testing = StateId::from("testing");
        let _production = StateId::from("production");
        let _deploy = ActivityId::from("deploy");
        let _rollback = ActivityId::from("rollback");

        // All are equally valid - the engine is completely generic
        assert!(cart != planning);
        assert!(development != executing);
        assert!(add_item != start_task);
    }
}
