// Petri Net Places and Transitions - formal state management foundations
//
// ## Petri Net Theory Overview
//
// Circuit Breaker implements **State Managed Workflows** using Petri Net mathematics.
// This provides formal guarantees about workflow correctness that DAG-based systems cannot offer.
//
// ### Core Concepts:
//
// **Places (PlaceId)**: Represent states in the workflow where tokens can reside.
// - Example: "draft", "review", "approved", "rejected"
// - Tokens indicate the current state of workflow execution
// - Multiple tokens can exist in different places simultaneously
//
// **Transitions (TransitionId)**: Represent actions that move tokens between places.
// - Example: "submit", "approve", "reject", "revise"
// - Can have multiple input places (synchronization)
// - Can have multiple output places (branching)
// - Enable cycles (impossible in DAGs)
//
// **Tokens**: Represent workflow instances executing through the Petri net.
// - Each token carries domain-specific data
// - Tokens move from place to place via transitions
// - History of all movements is preserved for audit trails
//
// ### State Management Advantages over DAGs:
//
// 1. **Cycles Supported**: Revision loops, retries, rollbacks are natural
// 2. **Concurrent Execution**: Multiple tokens in different places
// 3. **Synchronization**: Transitions can wait for multiple input tokens
// 4. **Mathematical Verification**: Formal analysis of deadlocks, liveness
// 5. **Flexible Branching**: Complex decision points with multiple outcomes
//
// ### Example Petri Net:
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
// - PlaceId and TransitionId are simple string wrappers for maximum flexibility
// - Any domain can define their own place and transition names
// - The engine is completely generic - no hardcoded workflow knowledge
// - GraphQL clients define domain-specific workflows at runtime

use serde::{Deserialize, Serialize};

/// **Petri Net Place** - represents a state where tokens can reside
/// 
/// In Petri net theory, a **place** is a location where tokens can exist.
/// Places represent conditions, states, or resources in the modeled system.
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
/// - **Generic**: Any string can be a place name
/// - **Domain-Agnostic**: No hardcoded business logic
/// - **Flexible**: Client applications define their own place semantics
/// - **Immutable**: Place identities don't change once created
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaceId(pub String);

impl PlaceId {
    /// Get the place identifier as a string slice
    /// 
    /// ```rust
    /// # use circuit_breaker::PlaceId;
    /// let place = PlaceId::from("draft");
    /// assert_eq!(place.as_str(), "draft");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a new place from any string-like input
    /// 
    /// ```rust
    /// # use circuit_breaker::PlaceId;
    /// let place1 = PlaceId::new("review");
    /// let place2 = PlaceId::from("review");
    /// assert_eq!(place1, place2);
    /// ```
    pub fn new<S: Into<String>>(name: S) -> Self {
        PlaceId(name.into())
    }
}

impl From<&str> for PlaceId {
    fn from(s: &str) -> Self {
        PlaceId(s.to_string())
    }
}

impl From<String> for PlaceId {
    fn from(s: String) -> Self {
        PlaceId(s)
    }
}

impl std::fmt::Display for PlaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// **Petri Net Transition** - represents an action that moves tokens between places
/// 
/// In Petri net theory, a **transition** is an active component that consumes tokens
/// from input places and produces tokens in output places. Transitions represent
/// actions, events, or computations in the modeled system.
/// 
/// ## Examples by Domain:
/// 
/// **Document Workflow**: "submit", "approve", "reject", "revise", "publish"
/// **Software Deployment**: "build", "test", "deploy", "rollback", "scale"
/// **Order Processing**: "add_to_cart", "checkout", "pay", "ship", "deliver"
/// **AI Agent Campaign**: "research", "generate", "review", "optimize", "publish"
///
/// ## Transition Firing Rules:
/// 
/// 1. **Preconditions**: All required input places must contain tokens
/// 2. **Action**: Transition fires, consuming input tokens  
/// 3. **Postconditions**: New tokens are produced in output places
/// 4. **Atomicity**: Either all changes happen or none (transaction semantics)
///
/// ## Advanced Patterns:
/// 
/// - **Synchronization**: Transition waits for tokens in multiple input places
/// - **Branching**: Transition produces tokens in multiple output places
/// - **Choice**: Multiple transitions compete for the same input tokens
/// - **Cycles**: Transitions can create loops (revision cycles, retries)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransitionId(pub String);

impl TransitionId {
    /// Get the transition identifier as a string slice
    /// 
    /// ```rust
    /// # use circuit_breaker::TransitionId;
    /// let transition = TransitionId::from("submit");
    /// assert_eq!(transition.as_str(), "submit");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a new transition from any string-like input
    /// 
    /// ```rust
    /// # use circuit_breaker::TransitionId;
    /// let trans1 = TransitionId::new("approve");
    /// let trans2 = TransitionId::from("approve");
    /// assert_eq!(trans1, trans2);
    /// ```
    pub fn new<S: Into<String>>(name: S) -> Self {
        TransitionId(name.into())
    }
}

impl From<&str> for TransitionId {
    fn from(s: &str) -> Self {
        TransitionId(s.to_string())
    }
}

impl From<String> for TransitionId {
    fn from(s: String) -> Self {
        TransitionId(s)
    }
}

impl std::fmt::Display for TransitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_id_creation() {
        let place1 = PlaceId::from("draft");
        let place2 = PlaceId::from("draft".to_string());
        let place3 = PlaceId("draft".to_string());
        let place4 = PlaceId::new("draft");
        
        assert_eq!(place1, place2);
        assert_eq!(place2, place3);
        assert_eq!(place3, place4);
        assert_eq!(place1.as_str(), "draft");
        assert_eq!(place1.to_string(), "draft");
    }

    #[test]
    fn test_transition_id_creation() {
        let trans1 = TransitionId::from("submit");
        let trans2 = TransitionId::from("submit".to_string());
        let trans3 = TransitionId::new("submit");
        
        assert_eq!(trans1, trans2);
        assert_eq!(trans2, trans3);
        assert_eq!(trans1.as_str(), "submit");
        assert_eq!(trans1.to_string(), "submit");
    }

    #[test]
    fn test_petri_net_example() {
        // Model a simple document review process
        let draft = PlaceId::from("draft");
        let review = PlaceId::from("review"); 
        let _approved = PlaceId::from("approved");
        let _rejected = PlaceId::from("rejected");
        
        let submit = TransitionId::from("submit");
        let approve = TransitionId::from("approve");
        let _reject = TransitionId::from("reject");
        let _revise = TransitionId::from("revise");
        
        // This Petri net supports cycles: draft -> review -> rejected -> draft
        // Which is impossible to model with DAGs!
        assert_ne!(draft, review);
        assert_ne!(submit, approve);
        
        // Places and transitions have meaningful string representations
        assert_eq!(format!("{}", draft), "draft");
        assert_eq!(format!("{}", submit), "submit");
    }
    
    #[test]
    fn test_generic_workflow_modeling() {
        // Demonstrate that any domain can be modeled
        
        // E-commerce workflow
        let cart = PlaceId::from("cart");
        let _checkout = PlaceId::from("checkout");
        let _paid = PlaceId::from("paid");
        let add_item = TransitionId::from("add_item");
        let _purchase = TransitionId::from("purchase");
        
        // AI agent workflow  
        let planning = PlaceId::from("planning");
        let executing = PlaceId::from("executing");
        let _complete = PlaceId::from("complete");
        let start_task = TransitionId::from("start_task");
        let _finish_task = TransitionId::from("finish_task");
        
        // Software deployment workflow
        let development = PlaceId::from("development");
        let _testing = PlaceId::from("testing");
        let _production = PlaceId::from("production");
        let _deploy = TransitionId::from("deploy");
        let _rollback = TransitionId::from("rollback");
        
        // All are equally valid - the engine is completely generic
        assert!(cart != planning);
        assert!(development != executing);
        assert!(add_item != start_task);
    }
} 