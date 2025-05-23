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

use serde::{Deserialize, Serialize}; // JSON serialization support
use super::place::{PlaceId, TransitionId}; // Basic Petri net components
use super::transition::TransitionDefinition; // Transition specifications

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
    
    /// All possible places (states) in this workflow
    /// This defines the complete state space - every valid state a token can be in
    /// Examples: ["draft", "review", "approved"], ["dev", "test", "prod"]
    pub places: Vec<PlaceId>,
    
    /// All possible transitions (actions) in this workflow
    /// This defines how tokens can move between places
    /// Each transition specifies source places, target place, and conditions
    pub transitions: Vec<TransitionDefinition>,
    
    /// The starting place where new tokens are created
    /// Every new token in this workflow begins here
    /// Must be one of the places in the `places` vector
    pub initial_place: PlaceId,
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
    /// - `I: Into<PlaceId>` - initial_place can be &str, String, or PlaceId
    /// 
    /// ### Ownership and Moving
    /// The `places` and `transitions` parameters are moved into the struct.
    /// This is efficient because we don't need to clone the vectors.
    /// 
    /// ### Builder Pattern Alternative
    /// This is a comprehensive constructor. In larger applications, you might
    /// see a builder pattern instead for more complex construction.
    pub fn new<S: Into<String>, N: Into<String>, I: Into<PlaceId>>(
        id: S,                              // Workflow unique identifier
        name: N,                            // Human-readable name
        places: Vec<PlaceId>,               // All possible states
        transitions: Vec<TransitionDefinition>, // All possible actions
        initial_place: I,                   // Starting state
    ) -> Self {
        WorkflowDefinition {
            id: id.into(),              // Convert to String
            name: name.into(),          // Convert to String
            places,                     // Move the vector
            transitions,                // Move the vector
            initial_place: initial_place.into(), // Convert to PlaceId
        }
    }

    /// Check if a transition is valid from the current place
    /// 
    /// This is the core method used by the workflow engine to determine if
    /// a token can fire a specific transition from its current place.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Iterator Chaining
    /// This method demonstrates Rust's powerful iterator system:
    /// - `.iter()` creates an iterator over the transitions
    /// - `.find()` searches for the first matching element
    /// - `.map()` transforms the result if found
    /// 
    /// ### Option<T> Return Type
    /// Returns `Some(&PlaceId)` if the transition is valid, `None` if not.
    /// This forces callers to handle both cases explicitly.
    /// 
    /// ### Reference Deref
    /// `*transition_id` dereferences the TransitionId for comparison.
    /// This is needed because we're comparing `&TransitionId` with `TransitionId`.
    pub fn can_transition(&self, from_place: &PlaceId, transition_id: &TransitionId) -> Option<&PlaceId> {
        self.transitions
            .iter()                     // Create iterator over transitions
            .find(|t| {                 // Find first transition that matches both:
                t.id == *transition_id &&              // Same transition ID
                t.from_places.contains(from_place)     // Can trigger from this place
            })
            .map(|t| &t.to_place)       // If found, return reference to target place
    }

    /// Get all available transitions from a given place
    /// 
    /// This returns all transitions that can be fired when a token is in
    /// the specified place. Useful for displaying available actions to users.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Filter and Collect Pattern
    /// This is a common functional programming pattern in Rust:
    /// - `.filter()` keeps only elements that match a condition
    /// - `.collect()` gathers the results into a new collection
    /// 
    /// ### Reference Collections
    /// Returns `Vec<&TransitionDefinition>` - a vector of references.
    /// This avoids cloning the transition definitions.
    pub fn available_transitions(&self, from_place: &PlaceId) -> Vec<&TransitionDefinition> {
        self.transitions
            .iter()                     // Iterate over all transitions
            .filter(|t| {              // Keep only transitions that...
                t.from_places.contains(from_place) // ...can trigger from this place
            })
            .collect()                  // Collect references into a vector
    }

    /// Validate that all transition references point to valid places
    /// 
    /// This performs static analysis of the workflow to ensure it's well-formed:
    /// - Initial place exists in the places list
    /// - All transition source places exist
    /// - All transition target places exist
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Result<T, E> for Error Handling
    /// Returns `Result<(), String>` where:
    /// - `Ok(())` means validation succeeded (unit type () means "nothing")
    /// - `Err(String)` means validation failed with an error message
    /// 
    /// ### HashSet for Efficient Lookup
    /// Creates a HashSet from the places vector for O(1) lookup performance.
    /// Much faster than calling `.contains()` on a vector repeatedly.
    /// 
    /// ### Early Return Pattern
    /// Uses `return Err(...)` to exit immediately when validation fails.
    /// This is a common error handling pattern in Rust.
    pub fn validate(&self) -> Result<(), String> {
        // Create a HashSet of places for efficient lookups
        // HashSet provides O(1) lookup vs O(n) for Vec.contains()
        let place_set: std::collections::HashSet<_> = self.places.iter().collect();
        
        // Check initial place exists in the places list
        if !place_set.contains(&self.initial_place) {
            return Err(format!(
                "Initial place '{}' not found in places", 
                self.initial_place.as_str()
            ));
        }
        
        // Validate all transitions reference valid places
        for transition in &self.transitions {
            // Check all source places exist
            for from_place in &transition.from_places {
                if !place_set.contains(from_place) {
                    return Err(format!(
                        "Transition '{}' references invalid from_place '{}'", 
                        transition.id.as_str(), 
                        from_place.as_str()
                    ));
                }
            }
            
            // Check target place exists
            if !place_set.contains(&transition.to_place) {
                return Err(format!(
                    "Transition '{}' references invalid to_place '{}'", 
                    transition.id.as_str(), 
                    transition.to_place.as_str()
                ));
            }
        }
        
        // If we get here, validation passed
        Ok(())
    }

    /// Get all places that can transition to the given place
    /// 
    /// This performs reverse lookup - given a target place, find all places
    /// that have transitions leading to it. Useful for workflow analysis.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### flat_map for Flattening
    /// `flat_map` is like `map` but flattens the results:
    /// - Each transition has multiple from_places (Vec<PlaceId>)
    /// - `flat_map` flattens all these vectors into a single iterator
    /// - Results in a flat list of all source places
    pub fn incoming_places(&self, to_place: &PlaceId) -> Vec<&PlaceId> {
        self.transitions
            .iter()                     // Iterate over transitions
            .filter(|t| t.to_place == *to_place)  // Keep only transitions to target place
            .flat_map(|t| &t.from_places)  // Flatten all from_places into single iterator
            .collect()                  // Collect into vector
    }

    /// Get all places that can be reached from the given place
    /// 
    /// This finds all target places reachable in one transition from the source.
    /// Different from `incoming_places` - this looks forward instead of backward.
    pub fn outgoing_places(&self, from_place: &PlaceId) -> Vec<&PlaceId> {
        self.transitions
            .iter()                     // Iterate over transitions
            .filter(|t| {              // Keep transitions that...
                t.from_places.contains(from_place) // ...can trigger from source place
            })
            .map(|t| &t.to_place)      // Extract target places
            .collect()                  // Collect into vector
    }

    /// Check if the workflow has any unreachable places
    /// 
    /// This implements a graph traversal algorithm to find places that can
    /// never be reached from the initial place. Such places indicate potential
    /// workflow design problems.
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Graph Traversal Algorithm
    /// This implements a depth-first search (DFS) to find reachable places:
    /// 1. Start with initial place
    /// 2. Mark it as reachable
    /// 3. Add all its outgoing places to visit queue
    /// 4. Repeat until no more places to visit
    /// 5. Any place not marked reachable is unreachable
    /// 
    /// ### Mutable Local Variables
    /// Uses `mut` to create mutable local variables for the algorithm.
    /// The workflow itself is not modified - only local state.
    /// 
    /// ### while let Pattern
    /// `while let Some(place) = to_visit.pop()` is a common pattern for
    /// processing all items in a collection until it's empty.
    pub fn find_unreachable_places(&self) -> Vec<&PlaceId> {
        // Track which places we've already visited
        let mut reachable = std::collections::HashSet::new();
        
        // Queue of places to visit (DFS stack)
        let mut to_visit = vec![&self.initial_place];
        
        // Depth-first search from initial place
        while let Some(place) = to_visit.pop() {
            // If we haven't seen this place before...
            if reachable.insert(place) {
                // Mark it as reachable and add its outgoing places to visit
                for next_place in self.outgoing_places(place) {
                    if !reachable.contains(next_place) {
                        to_visit.push(next_place);
                    }
                }
            }
        }
        
        // Return places that were never marked as reachable
        self.places
            .iter()                     // Iterate over all places
            .filter(|place| {          // Keep only places that...
                !reachable.contains(place) // ...were not marked reachable
            })
            .collect()                  // Collect into vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::transition::TransitionDefinition;

    #[test]
    fn test_workflow_validation() {
        // Create a simple linear workflow for testing
        let workflow = WorkflowDefinition::new(
            "test_workflow",            // Workflow ID
            "Test Workflow",            // Human readable name
            vec![                       // All places in the workflow
                PlaceId::from("start"),
                PlaceId::from("middle"),
                PlaceId::from("end"),
            ],
            vec![                       // All transitions in the workflow
                TransitionDefinition::new("go", vec!["start"], "middle"),
                TransitionDefinition::new("finish", vec!["middle"], "end"),
            ],
            "start"                     // Initial place
        );

        // Test that the workflow is valid
        assert!(workflow.validate().is_ok());
        
        // Test valid transition lookup
        assert_eq!(
            workflow.can_transition(&PlaceId::from("start"), &TransitionId::from("go")),
            Some(&PlaceId::from("middle"))
        );
        
        // Test invalid transition lookup  
        assert_eq!(
            workflow.can_transition(&PlaceId::from("start"), &TransitionId::from("finish")),
            None
        );
    }

    #[test]
    fn test_available_transitions() {
        // Create a workflow with branching (multiple transitions from one place)
        let workflow = WorkflowDefinition::new(
            "test_workflow",
            "Test Workflow",
            vec![
                PlaceId::from("draft"),
                PlaceId::from("review"),
                PlaceId::from("approved"),
                PlaceId::from("rejected"),
            ],
            vec![
                TransitionDefinition::new("submit", vec!["draft"], "review"),
                TransitionDefinition::new("approve", vec!["review"], "approved"),
                TransitionDefinition::new("reject", vec!["review"], "rejected"),
            ],
            "draft"
        );

        // Test that review place has two available transitions
        let available = workflow.available_transitions(&PlaceId::from("review"));
        assert_eq!(available.len(), 2); // approve and reject
        
        // Extract transition IDs for easier testing
        let transition_ids: Vec<&str> = available
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        
        // Verify both expected transitions are present
        assert!(transition_ids.contains(&"approve"));
        assert!(transition_ids.contains(&"reject"));
    }

    #[test]
    fn test_workflow_analysis() {
        // Create a workflow with an unreachable place for testing
        let workflow = WorkflowDefinition::new(
            "analysis_test",
            "Analysis Test",
            vec![
                PlaceId::from("start"),
                PlaceId::from("middle"),
                PlaceId::from("end"),
                PlaceId::from("orphan"), // This should be unreachable
            ],
            vec![
                TransitionDefinition::new("go", vec!["start"], "middle"),
                TransitionDefinition::new("finish", vec!["middle"], "end"),
                // Note: no transitions lead to or from "orphan"
            ],
            "start"
        );

        // Test incoming places analysis
        let incoming = workflow.incoming_places(&PlaceId::from("middle"));
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0], &PlaceId::from("start"));

        // Test outgoing places analysis
        let outgoing = workflow.outgoing_places(&PlaceId::from("middle"));
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0], &PlaceId::from("end"));

        // Test unreachable places detection
        let unreachable = workflow.find_unreachable_places();
        assert_eq!(unreachable.len(), 1);
        assert_eq!(unreachable[0], &PlaceId::from("orphan"));
    }
} 