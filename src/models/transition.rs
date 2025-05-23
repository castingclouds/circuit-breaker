// Transition definitions - workflow state change specifications

//! # Transition Definitions
//! 
//! This module defines how tokens can move between places in a workflow.
//! A `TransitionDefinition` specifies:
//! - Which places a token can come from (source places)
//! - Which place a token will go to (target place)
//! - What conditions must be met for the transition to fire
//! 
//! ## Petri Net Theory
//! 
//! In Petri Net terminology:
//! - **Input Places**: Places where tokens must be present for transition to fire
//! - **Output Place**: Place where tokens will be created after transition fires
//! - **Conditions**: Additional business logic that must be satisfied
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
/// 
/// **Software Deployment**:
/// - from_places: ["tested"] 
/// - to_place: "production"
/// - conditions: ["all_tests_pass", "security_approved"]
/// 
/// **Order Processing**:
/// - from_places: ["cart"]
/// - to_place: "payment_pending"
/// - conditions: ["items_available", "customer_verified"]
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
    
    /// Business logic conditions that must be satisfied
    /// These are domain-specific rules checked by the application
    /// Examples: ["has_content"], ["all_tests_pass", "security_scan_clean"]
    pub conditions: Vec<String>, // Generic condition strings
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
        }
    }

    /// Create a transition with conditions
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
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from the parent module

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
} 