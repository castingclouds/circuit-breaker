// Workflow definitions - complete state machine specifications

//! # Workflow Definitions
//!
//! This module defines complete workflow specifications using Petri Net theory.
//! A `WorkflowDefinition` is a complete state machine that describes:
//! - All possible states (places) in the workflow
//! - All possible actions (transitions) between states
//! - The starting state (initial place)
//! - Validation rules to ensure the workflow is well-formed
//!
//! ## Petri Net Theory
//!
//! A workflow definition is essentially a **Petri Net**:
//! - **Places**: States where tokens can reside (draft, review, approved, etc.)
//! - **Transitions**: Actions that move tokens between places (submit, approve, etc.)
//! - **Initial Marking**: Where tokens start (initial_place)
//!
//! ## Key Advantages over DAGs
//!
//! Unlike Directed Acyclic Graphs (DAGs), Petri Nets support:
//! - **Cycles**: Revision loops, retry mechanisms, rollbacks
//! - **Synchronization**: Waiting for multiple conditions
//! - **Concurrent Execution**: Multiple tokens in different places
//! - **Mathematical Verification**: Formal analysis of deadlocks, liveness
//!
//! ## Rust Learning Notes:
//!
//! This file demonstrates advanced Rust concepts:
//! - Iterator chaining and functional programming
//! - Graph algorithms (reachability analysis)
//! - Error handling with Result types
//! - Hash sets for efficient lookups
//! - Complex generic functions

use super::activity::ActivityDefinition;
use super::state::{ActivityId, StateId}; // Basic workflow components
use serde::{Deserialize, Serialize}; // JSON serialization support

/// Generic workflow definition - completely domain-agnostic
///
/// This represents a complete workflow that can be used for any domain:
/// document review, software deployment, order processing, AI agent pipelines, etc.
///
/// ## Design Philosophy
///
/// The workflow definition is **completely generic**:
/// - No hardcoded business logic
/// - No domain-specific knowledge
/// - Any client can define workflows via GraphQL
/// - The Rust engine just executes the abstract state machine
///
/// ## Examples:
///
/// **Document Review Workflow**:
/// - places: ["draft", "review", "approved", "rejected", "published"]
/// - initial_place: "draft"
/// - transitions: submit, approve, reject, revise, publish
///
/// **Software Deployment**:
/// - places: ["development", "testing", "staging", "production"]
/// - initial_place: "development"
/// - transitions: build, test, deploy, rollback
///
/// **Order Processing**:
/// - places: ["cart", "payment", "fulfillment", "shipped", "delivered"]
/// - initial_place: "cart"
/// - transitions: checkout, pay, ship, deliver
///
/// ## Rust Learning Notes:
///
/// ### Struct with Vector Fields
/// This struct contains several vectors, which are dynamically-sized arrays:
/// - `Vec<PlaceId>`: Can hold any number of places
/// - `Vec<TransitionDefinition>`: Can hold any number of transitions
///
/// ### Derive Macros for Common Functionality
/// The derive attributes provide standard functionality automatically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique identifier for this workflow definition
    /// Examples: "document_review", "deployment_pipeline", "order_fulfillment"
    pub id: String,

    /// Human-readable name for this workflow
    /// Examples: "Document Review Process", "CI/CD Pipeline", "E-commerce Order"
    pub name: String,

    /// All possible states in this workflow
    /// This defines the complete state space - every valid state a resource can be in
    /// Examples: ["draft", "review", "approved"], ["dev", "test", "prod"]
    pub states: Vec<StateId>,

    /// All possible activities (actions) in this workflow
    /// This defines how resources can move between states
    /// Each activity specifies source states, target state, and conditions
    pub activities: Vec<ActivityDefinition>,

    /// The starting state where new resources are created
    /// Every new resource in this workflow begins here
    /// Must be one of the states in the `states` vector
    pub initial_state: StateId,
}

impl WorkflowDefinition {
    /// Create a new workflow definition
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Multiple Generic Parameters
    /// This function accepts three different generic types:
    /// - `S: Into<String>` - id can be &str, String, or anything convertible to String
    /// - `N: Into<String>` - name can be &str, String, etc.
    /// - `I: Into<StateId>` - initial_state can be &str, String, or StateId
    ///
    /// ### Ownership and Moving
    /// The `states` and `activities` parameters are moved into the struct.
    /// This is efficient because we don't need to clone the vectors.
    ///
    /// ### Builder Pattern Alternative
    /// This is a comprehensive constructor. In larger applications, you might
    /// see a builder pattern instead for more complex construction.
    pub fn new<S: Into<String>, N: Into<String>, I: Into<StateId>>(
        id: S,                               // Workflow unique identifier
        name: N,                             // Human-readable name
        states: Vec<StateId>,                // All possible states
        activities: Vec<ActivityDefinition>, // All possible actions
        initial_state: I,                    // Starting state
    ) -> Self {
        WorkflowDefinition {
            id: id.into(),                       // Convert to String
            name: name.into(),                   // Convert to String
            states,                              // Move the vector
            activities,                          // Move the vector
            initial_state: initial_state.into(), // Convert to StateId
        }
    }

    /// Check if an activity is valid from the current state
    ///
    /// This is the core method used by the workflow engine to determine if
    /// a resource can execute a specific activity from its current state.
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Iterator Chaining
    /// This method demonstrates Rust's powerful iterator system:
    /// - `.iter()` creates an iterator over the activities
    /// - `.find()` searches for the first matching element
    /// - `.map()` transforms the result if found
    ///
    /// ### Option<T> Return Type
    /// Returns `Some(&StateId)` if the activity is valid, `None` if not.
    /// This forces callers to handle both cases explicitly.
    ///
    /// ### Reference Deref
    /// `*activity_id` dereferences the ActivityId for comparison.
    /// This is needed because we're comparing `&ActivityId` with `ActivityId`.
    pub fn can_execute_activity(
        &self,
        from_state: &StateId,
        activity_id: &ActivityId,
    ) -> Option<&StateId> {
        self.activities
            .iter() // Create iterator over activities
            .find(|a| {
                // Find first activity that matches both:
                a.id == *activity_id &&                // Same activity ID
                a.from_states.contains(from_state) // Can execute from this state
            })
            .map(|a| &a.to_state) // If found, return reference to target state
    }

    /// Get all available activities from a given state
    ///
    /// This returns all activities that can be executed when a resource is in
    /// the specified state. Useful for displaying available actions to users.
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Filter and Collect Pattern
    /// This is a common functional programming pattern in Rust:
    /// - `.filter()` keeps only elements that match a condition
    /// - `.collect()` gathers the results into a new collection
    ///
    /// ### Reference Collections
    /// Returns `Vec<&ActivityDefinition>` - a vector of references.
    /// This avoids cloning the activity definitions.
    pub fn available_activities(&self, from_state: &StateId) -> Vec<&ActivityDefinition> {
        self.activities
            .iter() // Iterate over all activities
            .filter(|a| {
                // Keep only activities that...
                a.from_states.contains(from_state) // ...can execute from this state
            })
            .collect() // Collect references into a vector
    }

    /// Validate that all activity references point to valid states
    ///
    /// This performs static analysis of the workflow to ensure it's well-formed:
    /// - Initial state exists in the states list
    /// - All activity source states exist
    /// - All activity target states exist
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Result<T, E> for Error Handling
    /// Returns `Result<(), String>` where:
    /// - `Ok(())` means validation succeeded (unit type () means "nothing")
    /// - `Err(String)` means validation failed with an error message
    ///
    /// ### HashSet for Efficient Lookup
    /// Creates a HashSet from the states vector for O(1) lookup performance.
    /// Much faster than calling `.contains()` on a vector repeatedly.
    ///
    /// ### Early Return Pattern
    /// Uses `return Err(...)` to exit immediately when validation fails.
    /// This is a common error handling pattern in Rust.
    pub fn validate(&self) -> Result<(), String> {
        // Create a HashSet of states for efficient lookups
        // HashSet provides O(1) lookup vs O(n) for Vec.contains()
        let state_set: std::collections::HashSet<_> = self.states.iter().collect();

        // Check initial state exists in the states list
        if !state_set.contains(&self.initial_state) {
            return Err(format!(
                "Initial state '{}' not found in states",
                self.initial_state.as_str()
            ));
        }

        // Validate all activities reference valid states
        for activity in &self.activities {
            // Check all source states exist
            for from_state in &activity.from_states {
                if !state_set.contains(from_state) {
                    return Err(format!(
                        "Activity '{}' references invalid from_state '{}'",
                        activity.id.as_str(),
                        from_state.as_str()
                    ));
                }
            }

            // Check target state exists
            if !state_set.contains(&activity.to_state) {
                return Err(format!(
                    "Activity '{}' references invalid to_state '{}'",
                    activity.id.as_str(),
                    activity.to_state.as_str()
                ));
            }
        }

        // If we get here, validation passed
        Ok(())
    }

    /// Get all states that can transition to the given state
    ///
    /// This performs reverse lookup - given a target state, find all states
    /// that have activities leading to it. Useful for workflow analysis.
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### flat_map for Flattening
    /// `flat_map` is like `map` but flattens the results:
    /// - Each activity has multiple from_states (Vec<StateId>)
    /// - `flat_map` iterates over each Vec<StateId> and flattens them
    /// - Results in a flat list of all source states
    pub fn incoming_states(&self, to_state: &StateId) -> Vec<&StateId> {
        self.activities
            .iter() // Iterate over activities
            .filter(|a| a.to_state == *to_state) // Keep only activities to target state
            .flat_map(|a| &a.from_states) // Flatten all from_states into single iterator
            .collect() // Collect into vector
    }

    /// Get all states that can be reached from the given state
    ///
    /// This finds all target states reachable in one activity from the source.
    /// Different from `incoming_states` - this looks forward instead of backward.
    pub fn outgoing_states(&self, from_state: &StateId) -> Vec<&StateId> {
        self.activities
            .iter() // Iterate over activities
            .filter(|a| {
                // Keep activities that...
                a.from_states.contains(from_state) // ...can execute from source state
            })
            .map(|a| &a.to_state) // Extract target states
            .collect() // Collect into vector
    }

    /// Check if the workflow has any unreachable states
    ///
    /// This implements a graph traversal algorithm to find states that can
    /// never be reached from the initial state. Such states indicate potential
    /// workflow design problems.
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Graph Traversal Algorithm
    /// This implements a depth-first search (DFS) to find reachable states:
    /// 1. Start with initial state
    /// 2. Mark it as reachable
    /// 3. Add all its outgoing states to visit queue
    /// 4. Repeat until no more states to visit
    /// 5. Any state not marked reachable is unreachable
    ///
    /// ### Mutable Local Variables
    /// Uses `mut` to create mutable local variables for the algorithm.
    /// The workflow itself is not modified - only local state.
    ///
    /// ### while let Pattern
    /// `while let Some(state) = to_visit.pop()` is a common pattern for
    /// processing all items in a collection until it's empty.
    pub fn find_unreachable_states(&self) -> Vec<&StateId> {
        // Track which states we've already visited
        let mut reachable = std::collections::HashSet::new();

        // Queue of states to visit (DFS stack)
        let mut to_visit = vec![&self.initial_state];

        // Depth-first search from initial state
        while let Some(state) = to_visit.pop() {
            // If we haven't seen this state before...
            if reachable.insert(state) {
                // ...mark it as reachable and add its outgoing states to queue
                for outgoing_state in self.outgoing_states(state) {
                    if !reachable.contains(outgoing_state) {
                        to_visit.push(outgoing_state);
                    }
                }
            }
        }

        // Return states that were never marked as reachable
        self.states
            .iter() // Iterate over all states
            .filter(|state| {
                // Keep only states that...
                !reachable.contains(state) // ...were not marked reachable
            })
            .collect() // Collect into vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::activity::ActivityDefinition;

    #[test]
    fn test_workflow_validation() {
        // Create a simple linear workflow for testing
        let workflow = WorkflowDefinition::new(
            "test_workflow", // Workflow ID
            "Test Workflow", // Human readable name
            vec![
                // All states in the workflow
                StateId::from("start"),
                StateId::from("middle"),
                StateId::from("end"),
            ],
            vec![
                // All activities in the workflow
                ActivityDefinition::new("go", vec!["start"], "middle"),
                ActivityDefinition::new("finish", vec!["middle"], "end"),
            ],
            "start", // Initial state
        );

        // Test that the workflow is valid
        assert!(workflow.validate().is_ok());

        // Test valid activity lookup
        assert_eq!(
            workflow.can_execute_activity(&StateId::from("start"), &ActivityId::from("go")),
            Some(&StateId::from("middle"))
        );

        // Test invalid activity lookup
        assert_eq!(
            workflow.can_execute_activity(&StateId::from("start"), &ActivityId::from("finish")),
            None
        );
    }

    #[test]
    fn test_available_activities() {
        // Create a workflow with branching activities
        let workflow = WorkflowDefinition::new(
            "branching_workflow",
            "Branching Test",
            vec![
                StateId::from("draft"),
                StateId::from("review"),
                StateId::from("approved"),
                StateId::from("rejected"),
            ],
            vec![
                ActivityDefinition::new("submit", vec!["draft"], "review"),
                ActivityDefinition::new("approve", vec!["review"], "approved"),
                ActivityDefinition::new("reject", vec!["review"], "rejected"),
            ],
            "draft",
        );

        // Test that review state has two available activities
        let available = workflow.available_activities(&StateId::from("review"));
        assert_eq!(available.len(), 2); // approve and reject

        // Extract activity IDs for easier testing
        let activity_ids: Vec<&str> = available.iter().map(|a| a.id.as_str()).collect();

        // Verify both expected activities are present
        assert!(activity_ids.contains(&"approve"));
        assert!(activity_ids.contains(&"reject"));
    }

    #[test]
    fn test_workflow_analysis() {
        // Create a workflow with unreachable states for testing
        let workflow = WorkflowDefinition::new(
            "analysis_test",
            "Analysis Test",
            vec![
                StateId::from("start"),
                StateId::from("middle"),
                StateId::from("end"),
                StateId::from("orphan"), // This state is unreachable
            ],
            vec![
                ActivityDefinition::new("go", vec!["start"], "middle"),
                ActivityDefinition::new("finish", vec!["middle"], "end"),
                // Note: no activities lead to or from "orphan"
            ],
            "start",
        );

        // Test incoming states analysis
        let incoming = workflow.incoming_states(&StateId::from("middle"));
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0], &StateId::from("start"));

        // Test outgoing states analysis
        let outgoing = workflow.outgoing_states(&StateId::from("middle"));
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0], &StateId::from("end"));

        // Test unreachable states detection
        let unreachable = workflow.find_unreachable_states();
        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0], &StateId::from("orphan"));
    }
}
