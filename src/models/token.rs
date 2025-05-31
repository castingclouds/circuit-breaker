// Token domain models - workflow execution state

//! # Token Models
//! 
//! This module defines the core types for workflow execution:
//! - `Token`: Represents a workflow instance moving through states
//! - `HistoryEvent`: Records each state transition for audit trails
//! - `TokenMetadata`: Generic key-value storage for additional data
//! 
//! ## Rust Learning Notes:
//! 
//! ### External Dependencies
//! Rust has a rich ecosystem of crates (libraries). This file demonstrates
//! how to use several important ones:
//! - `std::collections::HashMap`: Built-in hash map data structure
//! - `chrono`: Date/time handling with timezone support
//! - `serde`: JSON serialization/deserialization
//! - `uuid`: Universally unique identifier generation
//! 
//! ### Generic Programming
//! This file shows Rust's powerful generic system, which allows code
//! to work with many different types while maintaining type safety.

use std::collections::HashMap;   // Standard library hash map
use chrono::{DateTime, Utc};     // Date/time with UTC timezone support
use serde::{Deserialize, Serialize}; // JSON conversion traits
use uuid::Uuid;                  // UUID generation and handling

use super::place::{PlaceId, TransitionId}; // Import from sibling module

/// NATS-specific transition record for detailed transition tracking
/// 
/// This struct extends the basic HistoryEvent with NATS-specific metadata
/// for distributed workflow tracking and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// The place the token was in before the transition
    pub from_place: PlaceId,
    
    /// The place the token moved to after the transition
    pub to_place: PlaceId,
    
    /// Which transition was fired to cause this state change
    pub transition_id: TransitionId,
    
    /// When this transition occurred (UTC timestamp)
    pub timestamp: DateTime<Utc>,
    
    /// What triggered this transition (agent, function, manual, etc.)
    pub triggered_by: Option<String>,
    
    /// NATS sequence number when this transition was recorded
    pub nats_sequence: Option<u64>,
    
    /// Additional transition-specific metadata
    pub metadata: Option<serde_json::Value>,
}

/// Generic token metadata - key-value store for any additional data
/// 
/// ## Rust Learning Notes:
/// 
/// ### Type Aliases
/// This creates a more descriptive name for a complex type. Instead of writing
/// `HashMap<String, serde_json::Value>` everywhere, we can write `TokenMetadata`.
/// This makes the code more readable and easier to change later.
/// 
/// ### serde_json::Value
/// This is a JSON value that can be any JSON type: string, number, boolean,
/// array, object, or null. It's Rust's equivalent of "any JSON value".
pub type TokenMetadata = HashMap<String, serde_json::Value>;

/// History event tracking state transitions
/// 
/// Every time a token moves from one place to another, we create a HistoryEvent
/// to record exactly what happened. This provides a complete audit trail.
/// 
/// ## Rust Learning Notes:
/// 
/// ### Struct Definition
/// This struct uses several important Rust concepts:
/// - All fields are `pub` (public) so they can be accessed directly
/// - Uses external types (DateTime, custom types)
/// - Uses Option<T> for optional fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEvent {
    /// When this transition occurred (UTC timestamp)
    pub timestamp: DateTime<Utc>,
    
    /// Which transition was fired to cause this state change
    pub transition: TransitionId,
    
    /// The place the token was in before the transition
    pub from: PlaceId,
    
    /// The place the token moved to after the transition
    pub to: PlaceId,
    
    /// Optional data associated with this specific transition
    /// Using Option<T> means this can be None (no data) or Some(data)
    pub data: Option<serde_json::Value>,
}

/// Generic token - represents workflow execution state
/// 
/// A Token is the core concept in Circuit Breaker. It represents one instance
/// of a workflow execution - like a document going through review, an order
/// being processed, or a deployment moving through stages.
/// 
/// ## Design Philosophy
/// 
/// This token is **completely domain-agnostic**. It doesn't know about documents,
/// orders, deployments, or any specific business logic. It only knows about:
/// - Which workflow it belongs to
/// - Which place it's currently in  
/// - Generic data (JSON)
/// - Generic metadata (key-value pairs)
/// - History of transitions
/// 
/// This makes the engine truly generic - any domain can use it via GraphQL.
/// 
/// ## Rust Learning Notes:
/// 
/// ### Complex Struct with Multiple Types
/// This struct demonstrates many Rust concepts:
/// - Mix of owned types (String, Vec) and custom types (PlaceId)
/// - Generic JSON data storage
/// - Timestamps with timezone support
/// - Vector for dynamic-length history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Unique identifier for this token instance
    /// Uuid::new_v4() generates a random UUID (Version 4)
    pub id: Uuid,
    
    /// References which workflow definition this token is executing
    /// This is just a string ID - the actual workflow is stored separately
    pub workflow_id: String,
    
    /// Current place where this token resides
    /// This is the "current state" of the workflow execution
    pub place: PlaceId,
    
    /// Generic JSON data - can hold any domain-specific information
    /// Examples: order details, document content, deployment config, etc.
    pub data: serde_json::Value,
    
    /// Key-value metadata for additional information
    /// Examples: user who created it, priority level, tags, etc.
    pub metadata: TokenMetadata,
    
    /// When this token was first created
    pub created_at: DateTime<Utc>,
    
    /// When this token was last modified
    pub updated_at: DateTime<Utc>,
    
    /// Complete history of all state transitions
    /// This provides full audit trail of the token's journey
    pub history: Vec<HistoryEvent>,
    
    /// NATS-specific fields for streaming support
    /// These fields are optional to maintain backward compatibility
    
    /// NATS stream sequence number for this token message
    /// Used for ordering and deduplication in NATS streams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_sequence: Option<u64>,
    
    /// NATS timestamp when the message was stored
    /// Provides NATS-level timestamping for distributed consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_timestamp: Option<DateTime<Utc>>,
    
    /// NATS subject where this token is currently stored
    /// Format: workflows.{workflow_id}.places.{place_id}.tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_subject: Option<String>,
    
    /// Transition history specifically for NATS tracking
    /// Contains additional NATS-specific metadata for each transition
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transition_history: Vec<TransitionRecord>,
}

impl Token {
    /// Create a new token in the specified workflow and initial place
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Constructor Pattern
    /// Rust doesn't have constructors like other languages. Instead, we use
    /// associated functions (functions called on the type itself, not an instance).
    /// 
    /// ### Parameter Types
    /// - `workflow_id: &str` - Borrows a string slice (doesn't take ownership)
    /// - `initial_place: PlaceId` - Takes ownership of the PlaceId
    /// 
    /// ### Return Type
    /// `Self` refers to the type we're implementing (Token). This is a common
    /// pattern for constructor functions.
    pub fn new(workflow_id: &str, initial_place: PlaceId) -> Self {
        // Get the current timestamp - we'll use it for both created_at and updated_at
        let now = Utc::now();
        
        Token {
            // Generate a new random UUID for this token
            id: Uuid::new_v4(),
            
            // Convert the borrowed string to an owned String
            workflow_id: workflow_id.to_string(),
            
            // Move the initial_place into the struct
            place: initial_place,
            
            // Start with an empty JSON object for data
            // serde_json::Map::new() creates an empty JSON object {}
            data: serde_json::Value::Object(serde_json::Map::new()),
            
            // Start with an empty metadata HashMap
            metadata: HashMap::new(),
            
            // Set creation and update timestamps to now
            created_at: now,
            updated_at: now,
            
            // Start with empty history - no transitions yet
            history: vec![], // vec![] is a macro to create an empty vector
            
            // Initialize NATS fields as None/empty for backward compatibility
            nats_sequence: None,
            nats_timestamp: None,
            nats_subject: None,
            transition_history: vec![],
        }
    }

    /// Get the current place as a string
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Borrowing and References
    /// - `&self` means this method borrows the Token (doesn't take ownership)
    /// - Returns `&str` which is a borrowed string slice (no allocation needed)
    /// - The caller can read the place but can't modify the Token
    /// 
    /// ### Method vs Associated Function
    /// This is a method because it takes `&self`. Methods are called with
    /// dot notation: `token.current_place()`
    pub fn current_place(&self) -> &str {
        // Call the as_str() method on the PlaceId to get a string slice
        self.place.as_str()
    }

    /// Transition to a new place, recording the transition in history
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Mutable References
    /// - `&mut self` means this method can modify the Token
    /// - The caller must have a mutable reference to call this method
    /// - Rust's borrow checker ensures memory safety
    /// 
    /// ### Ownership and Cloning
    /// We clone the old place to store in history because we're about to
    /// overwrite self.place with the new place.
    pub fn transition_to(&mut self, new_place: PlaceId, transition_id: TransitionId) {
        // Clone the current place before we overwrite it
        // .clone() creates a new copy of the PlaceId
        let old_place = self.place.clone();
        
        // Record the transition in history
        // This creates a new HistoryEvent struct
        let history_event = HistoryEvent {
            timestamp: Utc::now(),                // Current timestamp
            transition: transition_id,           // Which transition fired
            from: old_place,                     // Where we came from
            to: new_place.clone(),               // Where we're going (clone because we use it twice)
            data: None, // Could be populated with transition-specific data
        };
        
        // Add the history event to our history vector
        // .push() adds an element to the end of a vector
        self.history.push(history_event);
        
        // Update the token's current place
        self.place = new_place;
        
        // Update the modification timestamp
        self.updated_at = Utc::now();
    }

    /// Set metadata value
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Generic Parameters with Trait Bounds
    /// `<K: Into<String>>` means:
    /// - K is a generic type parameter
    /// - K must implement the Into<String> trait
    /// - This allows passing &str, String, or any type that can become a String
    /// 
    /// ### Trait Bounds
    /// The `Into<String>` trait bound ensures type safety while providing
    /// flexibility. The caller can pass "key", "key".to_string(), or any
    /// type that can be converted to String.
    pub fn set_metadata<K: Into<String>>(&mut self, key: K, value: serde_json::Value) {
        // Convert the key to a String and insert into the HashMap
        // .into() calls the Into<String> trait method
        self.metadata.insert(key.into(), value);
        
        // Update the modification timestamp
        self.updated_at = Utc::now();
    }

    /// Get metadata value
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Option Type for Safe Null Handling
    /// Rust doesn't have null pointers. Instead, it uses Option<T>:
    /// - `Some(value)` means the value exists
    /// - `None` means no value (like null in other languages)
    /// 
    /// ### HashMap.get() Returns Option
    /// HashMap's get() method returns Option<&V> because the key might not exist.
    /// This forces callers to handle the "not found" case explicitly.
    /// 
    /// ### Reference Types
    /// Returns `Option<&serde_json::Value>` - a reference to the value if it exists.
    /// This avoids copying the value, which is more efficient.
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        // HashMap.get() returns Option<&V> where V is the value type
        self.metadata.get(key)
    }

    /// Check if token is in a specific place
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Comparison with References
    /// We're comparing `&self.place` with `place` (which is &PlaceId).
    /// The == operator works with references because PlaceId implements PartialEq.
    /// 
    /// ### Boolean Return Type
    /// Rust functions return the last expression if there's no semicolon.
    /// `&self.place == place` evaluates to a bool, which is returned.
    pub fn is_in_place(&self, place: &PlaceId) -> bool {
        // Compare the token's current place with the provided place
        &self.place == place
    }

    /// Get the last transition (most recent history event)
    /// 
    /// ## Rust Learning Notes:
    /// 
    /// ### Vector Methods
    /// - `.last()` returns Option<&T> - the last element if the vector isn't empty
    /// - Returns a reference to avoid copying the HistoryEvent
    /// 
    /// ### Chaining Option Methods
    /// Since .last() returns Option, callers can use methods like:
    /// - `token.last_transition().map(|event| &event.transition)`
    /// - `if let Some(event) = token.last_transition() { ... }`
    pub fn last_transition(&self) -> Option<&HistoryEvent> {
        // Vec.last() returns Option<&T> for the last element
        self.history.last()
    }

    /// NATS-specific methods for streaming support
    
    /// Set NATS metadata for this token
    pub fn set_nats_metadata(&mut self, sequence: u64, timestamp: DateTime<Utc>, subject: String) {
        self.nats_sequence = Some(sequence);
        self.nats_timestamp = Some(timestamp);
        self.nats_subject = Some(subject);
        self.updated_at = Utc::now();
    }
    
    /// Get NATS subject for this token's current place
    pub fn nats_subject_for_place(&self) -> String {
        format!("cb.workflows.{}.places.{}.tokens.{}", self.workflow_id, self.place.as_str(), self.id)
    }
    
    /// Add a transition record for NATS tracking
    pub fn add_transition_record(&mut self, record: TransitionRecord) {
        self.transition_history.push(record);
        self.updated_at = Utc::now();
    }
    
    /// Create a transition record from a regular transition
    pub fn create_transition_record(
        &self,
        from: PlaceId,
        to: PlaceId,
        transition_id: TransitionId,
        triggered_by: Option<String>,
        nats_sequence: Option<u64>,
    ) -> TransitionRecord {
        TransitionRecord {
            from_place: from,
            to_place: to,
            transition_id,
            timestamp: Utc::now(),
            triggered_by,
            nats_sequence,
            metadata: None,
        }
    }
    
    /// Enhanced transition method that includes NATS tracking
    pub fn transition_to_with_nats(
        &mut self, 
        new_place: PlaceId, 
        transition_id: TransitionId,
        triggered_by: Option<String>,
        nats_sequence: Option<u64>,
    ) {
        let old_place = self.place.clone();
        
        // Create both regular history event and NATS transition record
        let history_event = HistoryEvent {
            timestamp: Utc::now(),
            transition: transition_id.clone(),
            from: old_place.clone(),
            to: new_place.clone(),
            data: None,
        };
        
        let transition_record = self.create_transition_record(
            old_place,
            new_place.clone(),
            transition_id,
            triggered_by,
            nats_sequence,
        );
        
        // Update token state
        self.history.push(history_event);
        self.transition_history.push(transition_record);
        self.place = new_place;
        self.updated_at = Utc::now();
        
        // Update NATS subject for new place
        self.nats_subject = Some(self.nats_subject_for_place());
    }
    
    /// Check if this token has NATS metadata
    pub fn has_nats_metadata(&self) -> bool {
        self.nats_sequence.is_some() && self.nats_timestamp.is_some()
    }
    
    /// Get the most recent transition record
    pub fn last_transition_record(&self) -> Option<&TransitionRecord> {
        self.transition_history.last()
    }
}

#[cfg(test)]
mod tests {
    // Import everything from the parent module (the token module)
    // This is a common pattern in test modules
    use super::*;

    #[test]
    fn test_generic_token() {
        // Test with any domain - this could be a document, deployment, order, etc.
        // The token is completely generic and domain-agnostic
        let mut token = Token::new("my_custom_workflow", PlaceId::from("initial_place"));
        
        // Test the basic properties
        assert_eq!(token.current_place(), "initial_place");
        assert_eq!(token.workflow_id, "my_custom_workflow");
        assert!(token.history.is_empty()); // Vec.is_empty() checks if length is 0
        
        // Set some generic data using the json! macro
        // This macro creates serde_json::Value from JSON syntax
        token.data = serde_json::json!({
            "title": "My Thing",
            "priority": "high"
        });
        
        // Set metadata using the generic method
        token.set_metadata("department", serde_json::json!("engineering"));
        token.set_metadata("assignee", serde_json::json!("user@example.com"));
        
        // Test metadata retrieval
        assert_eq!(
            token.get_metadata("department"), 
            Some(&serde_json::json!("engineering"))
        );
    }

    #[test]
    fn test_generic_transition() {
        let mut token = Token::new("any_workflow", PlaceId::from("start"));
        
        // Transition using any place names - completely generic
        token.transition_to(PlaceId::from("middle"), TransitionId::from("advance"));
        
        // Verify the transition worked
        assert_eq!(token.current_place(), "middle");
        assert_eq!(token.history.len(), 1); // Vec.len() returns the number of elements
        
        // Test history tracking
        let last_transition = token.last_transition().unwrap(); // .unwrap() gets Some(value)
        assert_eq!(last_transition.from.as_str(), "start");
        assert_eq!(last_transition.to.as_str(), "middle");
        assert_eq!(last_transition.transition.as_str(), "advance");
        
        // Another transition to test multiple history events
        token.transition_to(PlaceId::from("end"), TransitionId::from("complete"));
        
        assert_eq!(token.current_place(), "end");
        assert_eq!(token.history.len(), 2);
    }

    #[test]
    fn test_place_checking() {
        let token = Token::new("test_workflow", PlaceId::from("draft"));
        
        // Test the is_in_place helper method
        assert!(token.is_in_place(&PlaceId::from("draft")));
        assert!(!token.is_in_place(&PlaceId::from("published")));
    }
} 