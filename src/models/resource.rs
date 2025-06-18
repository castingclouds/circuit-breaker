// Resource domain models - workflow execution state

//! # Resource Models
//!
//! This module defines the core types for workflow execution:
//! - `Resource`: Represents a workflow instance moving through states
//! - `HistoryEvent`: Records each state transition for audit trails
//! - `ResourceMetadata`: Generic key-value storage for additional data
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

use chrono::{DateTime, Utc}; // Date/time with UTC timezone support
use serde::{Deserialize, Serialize}; // JSON conversion traits
use std::collections::HashMap; // Standard library hash map
use uuid::Uuid; // UUID generation and handling

use super::state::{ActivityId, StateId}; // Import from sibling module

/// NATS-specific activity record for detailed activity tracking
///
/// This struct extends the basic HistoryEvent with NATS-specific metadata
/// for distributed workflow tracking and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    /// The state the resource was in before the activity
    pub from_state: StateId,

    /// The state the resource moved to after the activity
    pub to_state: StateId,

    /// Which activity was executed to cause this state change
    pub activity_id: ActivityId,

    /// When this activity occurred (UTC timestamp)
    pub timestamp: DateTime<Utc>,

    /// What triggered this activity (agent, function, manual, etc.)
    pub triggered_by: Option<String>,

    /// NATS sequence number when this activity was recorded
    pub nats_sequence: Option<u64>,

    /// Additional activity-specific metadata
    pub metadata: Option<serde_json::Value>,
}

/// Generic resource metadata - key-value store for any additional data
///
/// ## Rust Learning Notes:
///
/// ### Type Aliases
/// This creates a more descriptive name for a complex type. Instead of writing
/// `HashMap<String, serde_json::Value>` everywhere, we can write `ResourceMetadata`.
/// This makes the code more readable and easier to change later.
///
/// ### serde_json::Value
/// This is a JSON value that can be any JSON type: string, number, boolean,
/// array, object, or null. It's Rust's equivalent of "any JSON value".
pub type ResourceMetadata = HashMap<String, serde_json::Value>;

/// History event tracking state transitions
///
/// Every time a resource moves from one state to another, we create a HistoryEvent
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

    /// Which activity was executed to cause this state change
    pub activity: ActivityId,

    /// The state the resource was in before the activity
    pub from: StateId,

    /// The state the resource moved to after the activity
    pub to: StateId,

    /// Optional data associated with this specific transition
    /// Using Option<T> means this can be None (no data) or Some(data)
    pub data: Option<serde_json::Value>,
}

/// Generic resource - represents workflow execution state
///
/// A Resource is the core concept in Circuit Breaker. It represents one instance
/// of a workflow execution - like a document going through review, an order
/// being processed, or a deployment moving through stages.
///
/// ## Design Philosophy
///
/// This resource is **completely domain-agnostic**. It doesn't know about documents,
/// orders, deployments, or any specific business logic. It only knows about:
/// - Which workflow it belongs to
/// - Which state it's currently in
/// - Generic data (JSON)
/// - Generic metadata (key-value pairs)
/// - History of activities
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
pub struct Resource {
    /// Unique identifier for this resource instance
    /// Uuid::new_v4() generates a random UUID (Version 4)
    pub id: Uuid,

    /// References which workflow definition this resource is executing
    /// This is just a string ID - the actual workflow is stored separately
    pub workflow_id: String,

    /// Current state where this resource resides
    /// This is the "current state" of the workflow execution
    pub state: StateId,

    /// Generic JSON data - can hold any domain-specific information
    /// Examples: order details, document content, deployment config, etc.
    pub data: serde_json::Value,

    /// Key-value metadata for additional information
    /// Examples: user who created it, priority level, tags, etc.
    pub metadata: ResourceMetadata,

    /// When this resource was first created
    pub created_at: DateTime<Utc>,

    /// When this resource was last modified
    pub updated_at: DateTime<Utc>,

    /// Complete history of all state transitions
    /// This provides full audit trail of the resource's journey
    pub history: Vec<HistoryEvent>,

    /// NATS-specific fields for streaming support
    /// These fields are optional to maintain backward compatibility

    /// NATS stream sequence number for this resource message
    /// Used for ordering and deduplication in NATS streams
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_sequence: Option<u64>,

    /// NATS timestamp when the message was stored
    /// Provides NATS-level timestamping for distributed consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_timestamp: Option<DateTime<Utc>>,

    /// NATS subject where this resource is currently stored
    /// Format: workflows.{workflow_id}.states.{state_id}.resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats_subject: Option<String>,

    /// Activity history specifically for NATS tracking
    /// Contains additional NATS-specific metadata for each activity
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub activity_history: Vec<ActivityRecord>,
}

impl Resource {
    /// Create a new resource in the specified workflow and initial state
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Constructor Pattern
    /// Rust doesn't have constructors like other languages. Instead, we use
    /// associated functions (functions called on the type itself, not an instance).
    ///
    /// ### Parameter Types
    /// - `workflow_id: &str` - Borrows a string slice (doesn't take ownership)
    /// - `initial_state: StateId` - Takes ownership of the StateId
    ///
    /// ### Return Type
    /// `Self` refers to the type we're implementing (Resource). This is a common
    /// pattern for constructor functions.
    pub fn new(workflow_id: &str, initial_state: StateId) -> Self {
        // Get the current timestamp - we'll use it for both created_at and updated_at
        let now = Utc::now();

        Resource {
            // Generate a new random UUID for this resource
            id: Uuid::new_v4(),

            // Convert the borrowed string to an owned String
            workflow_id: workflow_id.to_string(),

            // Move the initial_state into the struct
            state: initial_state,

            // Start with an empty JSON object for data
            // serde_json::Map::new() creates an empty JSON object {}
            data: serde_json::Value::Object(serde_json::Map::new()),

            // Start with an empty metadata HashMap
            metadata: HashMap::new(),

            // Set creation and update timestamps to now
            created_at: now,
            updated_at: now,

            // Start with empty history - no activities yet
            history: vec![], // vec![] is a macro to create an empty vector

            // Initialize NATS fields as None/empty for backward compatibility
            nats_sequence: None,
            nats_timestamp: None,
            nats_subject: None,
            activity_history: vec![],
        }
    }

    /// Get the current state as a string
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Borrowing and References
    /// - `&self` means this method borrows the Resource (doesn't take ownership)
    /// - Returns `&str` which is a borrowed string slice (no allocation needed)
    /// - The caller can read the state but can't modify the Resource
    ///
    /// ### Method vs Associated Function
    /// This is a method because it takes `&self`. Methods are called with
    /// dot notation: `resource.current_state()`
    pub fn current_state(&self) -> &str {
        self.state.as_str()
    }

    /// Execute activity to move to a new state, recording the activity in history
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Mutable References
    /// - `&mut self` means this method can modify the Resource
    /// - The caller must have a mutable reference to call this method
    /// - Rust's borrow checker ensures memory safety
    ///
    /// ### Ownership and Cloning
    /// We clone the old state to store in history because we're about to
    /// overwrite self.state with the new state.
    pub fn execute_activity(&mut self, new_state: StateId, activity_id: ActivityId) {
        // Clone the current state before we overwrite it
        // .clone() creates a new copy of the StateId
        let old_state = self.state.clone();

        // Record the activity in history
        // This creates a new HistoryEvent struct
        let history_event = HistoryEvent {
            timestamp: Utc::now(), // Current timestamp
            activity: activity_id, // Which activity was executed
            from: old_state,       // Where we came from
            to: new_state.clone(), // Where we're going (clone because we use it twice)
            data: None,            // Could be populated with activity-specific data
        };

        // Add the history event to our history vector
        // .push() adds an element to the end of a vector
        self.history.push(history_event);

        // Update the resource's current state
        self.state = new_state;

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

    /// Check if resource is in a specific state
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Comparison with References
    /// We're comparing `&self.state` with `state` (which is &StateId).
    /// The == operator works with references because StateId implements PartialEq.
    ///
    /// ### Boolean Return Type
    /// Rust functions return the last expression if there's no semicolon.
    /// `&self.state == state` evaluates to a bool, which is returned.
    pub fn is_in_state(&self, state: &StateId) -> bool {
        &self.state == state
    }

    /// Get the last activity (most recent history event)
    ///
    /// ## Rust Learning Notes:
    ///
    /// ### Vector Methods
    /// - `.last()` returns Option<&T> - the last element if the vector isn't empty
    /// - Returns a reference to avoid copying the HistoryEvent
    ///
    /// ### Chaining Option Methods
    /// Since .last() returns Option, callers can use methods like:
    /// - `resource.last_activity().map(|event| &event.activity)`
    /// - `resource.last_activity().unwrap_or(&default_event)`
    pub fn last_activity(&self) -> Option<&HistoryEvent> {
        self.history.last()
    }

    /// NATS-specific methods for streaming support

    /// Set NATS metadata for this resource
    pub fn set_nats_metadata(&mut self, sequence: u64, timestamp: DateTime<Utc>, subject: String) {
        self.nats_sequence = Some(sequence);
        self.nats_timestamp = Some(timestamp);
        self.nats_subject = Some(subject);
        self.updated_at = Utc::now();
    }

    /// Get NATS subject for this resource's current state
    pub fn nats_subject_for_state(&self) -> String {
        format!(
            "cb.workflows.{}.states.{}.resources.{}",
            self.workflow_id,
            self.state.as_str(),
            self.id
        )
    }

    /// Add an activity record for NATS tracking
    pub fn add_activity_record(&mut self, record: ActivityRecord) {
        self.activity_history.push(record);
        self.updated_at = Utc::now();
    }

    /// Create an activity record from a regular activity
    pub fn create_activity_record(
        &self,
        from_state: StateId,
        to_state: StateId,
        activity_id: ActivityId,
        triggered_by: Option<String>,
        nats_sequence: Option<u64>,
    ) -> ActivityRecord {
        ActivityRecord {
            from_state,
            to_state,
            activity_id,
            timestamp: Utc::now(),
            triggered_by,
            nats_sequence,
            metadata: None,
        }
    }

    /// Enhanced activity execution method that includes NATS tracking
    pub fn execute_activity_with_nats(
        &mut self,
        new_state: StateId,
        activity_id: ActivityId,
        triggered_by: Option<String>,
        nats_sequence: Option<u64>,
    ) {
        let old_state = self.state.clone();

        // Create both regular history event and NATS activity record
        let history_event = HistoryEvent {
            timestamp: Utc::now(),
            activity: activity_id.clone(),
            from: old_state.clone(),
            to: new_state.clone(),
            data: None,
        };

        let activity_record = self.create_activity_record(
            old_state,
            new_state.clone(),
            activity_id,
            triggered_by,
            nats_sequence,
        );

        // Update resource state
        self.history.push(history_event);
        self.activity_history.push(activity_record);
        self.state = new_state;
        self.updated_at = Utc::now();

        // Update NATS subject for new state
        self.nats_subject = Some(self.nats_subject_for_state());
    }

    /// Check if this resource has NATS metadata
    pub fn has_nats_metadata(&self) -> bool {
        self.nats_sequence.is_some() && self.nats_timestamp.is_some()
    }

    /// Get the most recent activity record
    pub fn last_activity_record(&self) -> Option<&ActivityRecord> {
        self.activity_history.last()
    }
}

#[cfg(test)]
mod tests {
    // Import everything from the parent module (the token module)
    // This is a common pattern in test modules
    use super::*;

    #[test]
    fn test_generic_resource() {
        // Test with any domain - this could be a document, deployment, order, etc.
        // The resource is completely generic and domain-agnostic
        let mut resource = Resource::new("my_custom_workflow", StateId::from("initial_state"));

        // Test the basic properties
        assert_eq!(resource.current_state(), "initial_state");
        assert_eq!(resource.workflow_id, "my_custom_workflow");
        assert!(resource.history.is_empty()); // Vec.is_empty() checks if length is 0

        // Set some generic data using the json! macro
        // This macro creates serde_json::Value from JSON syntax
        resource.data = serde_json::json!({
            "title": "My Thing",
            "priority": "high"
        });

        // Set metadata using the generic method
        resource.set_metadata("department", serde_json::json!("engineering"));
        resource.set_metadata("assignee", serde_json::json!("user@example.com"));

        // Test metadata retrieval
        assert_eq!(
            resource.get_metadata("department"),
            Some(&serde_json::json!("engineering"))
        );
    }

    #[test]
    fn test_generic_activity() {
        let mut resource = Resource::new("any_workflow", StateId::from("start"));

        // Execute activity using any state names - completely generic
        resource.execute_activity(StateId::from("middle"), ActivityId::from("advance"));

        // Verify the activity worked
        assert_eq!(resource.current_state(), "middle");
        assert_eq!(resource.history.len(), 1); // Vec.len() returns the number of elements

        // Test history tracking
        let last_activity = resource.last_activity().unwrap(); // .unwrap() gets Some(value)
        assert_eq!(last_activity.from.as_str(), "start");
        assert_eq!(last_activity.to.as_str(), "middle");
        assert_eq!(last_activity.activity.as_str(), "advance");

        // Another activity to test multiple history events
        resource.execute_activity(StateId::from("end"), ActivityId::from("complete"));

        assert_eq!(resource.current_state(), "end");
        assert_eq!(resource.history.len(), 2);
    }

    #[test]
    fn test_state_checking() {
        let resource = Resource::new("test_workflow", StateId::from("draft"));

        // Test the is_in_state helper method
        assert!(resource.is_in_state(&StateId::from("draft")));
        assert!(!resource.is_in_state(&StateId::from("published")));
    }
}
