// Storage abstraction for the workflow engine
// This defines the interface for persisting workflows and resources

//! # Storage Abstraction Layer
//!
//! This module provides a storage abstraction that allows the workflow engine
//! to persist workflows and resources using different backends. The abstraction
//! separates business logic from storage implementation details.
//!
//! ## Storage Architecture
//!
//! The storage layer follows the **Repository Pattern**:
//! - **WorkflowStorage trait**: Defines the interface for all storage operations
//! - **InMemoryStorage**: Default implementation for development/testing
//! - **Future implementations**: NATS, PostgreSQL, Redis, etc.
//!
//! ## Async Design
//!
//! All storage operations are async to support:
//! - Non-blocking I/O operations
//! - Database connection pooling
//! - Network-based storage backends
//! - High concurrency scenarios
//!
//! ## Thread Safety
//!
//! The storage implementations must be thread-safe:
//! - Multiple async tasks can access storage concurrently
//! - Uses RwLock for safe concurrent access to in-memory data
//! - Send + Sync bounds ensure safe sharing across threads
//!
//! ## Rust Learning Notes:
//!
//! This file demonstrates advanced Rust concepts:
//! - Async traits with the async-trait crate
//! - Thread-safe concurrent data structures (RwLock)
//! - Trait objects and dynamic dispatch
//! - Error handling with Result types
//! - Option types for nullable database results

use std::collections::HashMap; // Hash map for key-value storage
use uuid::Uuid; // UUID type for token IDs

use crate::models::{Resource, WorkflowDefinition}; // Domain models
use crate::Result; // Custom Result type with our error types

/// Storage trait for workflow and resource persistence
///
/// This trait defines the interface that all storage backends must implement.
/// It provides a complete CRUD (Create, Read, Update, Delete) API for both
/// workflows and resources.
///
/// ## Design Principles
///
/// - **Async by Default**: All operations return futures for non-blocking I/O
/// - **Result-Based**: All operations can fail and return Result types
/// - **Generic**: Works with any storage backend (memory, database, network)
/// - **Thread-Safe**: Send + Sync bounds allow sharing across async tasks
///
/// ## Rust Learning Notes:
///
/// ### Async Traits
/// Rust doesn't natively support async functions in traits yet.
/// The `async-trait` crate provides a macro to enable async trait methods.
///
/// ### Trait Bounds
/// - `Send`: Type can be safely moved between threads
/// - `Sync`: Type can be safely shared between threads via references
/// These bounds are required for async trait objects.
///
/// ### Generic Return Types
/// The trait uses generic return types like `Result<Option<T>>`:
/// - `Result<T, E>`: Operation can succeed (Ok) or fail (Err)
/// - `Option<T>`: Value can exist (Some) or not exist (None)
/// - Combined: `Result<Option<T>>` means "operation can fail, and if it
///   succeeds, the value might or might not exist"
#[async_trait::async_trait]
pub trait WorkflowStorage: Send + Sync {
    /// Create a new workflow definition
    ///
    /// Stores a workflow definition and returns it back (possibly with
    /// modifications like generated timestamps, IDs, etc.)
    ///
    /// ## Errors
    /// - Workflow with same ID already exists
    /// - Storage backend is unavailable
    /// - Validation fails
    async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowDefinition>;

    /// Get a workflow definition by ID
    ///
    /// Returns `Some(workflow)` if found, `None` if not found.
    /// The operation itself can still fail (network error, etc.)
    ///
    /// ## Return Value
    /// `Result<Option<WorkflowDefinition>>` means:
    /// - `Ok(Some(workflow))`: Found the workflow
    /// - `Ok(None)`: No workflow with that ID (not an error)
    /// - `Err(error)`: Operation failed (storage error, network issue, etc.)
    async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDefinition>>;

    /// List all workflow definitions
    ///
    /// Returns all workflows in the storage. In production systems,
    /// this might be paginated or filtered.
    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>>;

    /// Create a new resource
    ///
    /// Stores a resource and returns it back. The resource ID is generated
    /// by the domain model, not the storage layer.
    async fn create_resource(&self, resource: Resource) -> Result<Resource>;

    /// Get a resource by ID
    ///
    /// Same pattern as get_workflow - returns Option wrapped in Result.
    async fn get_resource(&self, id: &Uuid) -> Result<Option<Resource>>;

    /// Update an existing resource
    ///
    /// Replaces the stored resource with the new version. This is used
    /// when resources execute activities between states or metadata is updated.
    async fn update_resource(&self, resource: Resource) -> Result<Resource>;

    /// List resources, optionally filtered by workflow
    ///
    /// If workflow_id is Some, returns only resources for that workflow.
    /// If workflow_id is None, returns all resources.
    ///
    /// ## Parameters
    /// - `workflow_id: Option<&str>`: Optional filter by workflow ID
    async fn list_resources(&self, workflow_id: Option<&str>) -> Result<Vec<Resource>>;
}

/// In-memory storage implementation for development and testing
///
/// This provides a simple in-memory implementation of the WorkflowStorage trait.
/// It's perfect for:
/// - Development and testing
/// - Demos and prototypes
/// - Unit tests
/// - Single-process deployments
///
/// ## Limitations
///
/// - **Not persistent**: Data is lost when process restarts
/// - **Not distributed**: Cannot share data across multiple processes
/// - **Memory bound**: Limited by available RAM
/// - **Not durable**: No backup or recovery mechanisms
///
/// ## Thread Safety
///
/// Uses `RwLock` for thread-safe concurrent access:
/// - Multiple readers can access data simultaneously
/// - Only one writer can modify data at a time
/// - Readers are blocked while writing occurs
///
/// ## Rust Learning Notes:
///
/// ### Default Derive
/// `#[derive(Default)]` automatically implements Default::default()
/// which creates empty HashMaps wrapped in RwLocks.
///
/// ### RwLock for Concurrent Access
/// `RwLock<T>` provides reader-writer lock semantics:
/// - `.read()` gets a read-only guard (multiple readers allowed)
/// - `.write()` gets a mutable guard (exclusive access)
/// - Guards automatically unlock when dropped (RAII pattern)
///
/// ### Interior Mutability Pattern
/// Even though the struct fields are not `mut`, we can still modify
/// the data inside through `RwLock`. This is called "interior mutability".
#[derive(Default)]
pub struct InMemoryStorage {
    /// Thread-safe storage for workflow definitions
    /// Key: workflow ID (String), Value: workflow definition
    workflows: std::sync::RwLock<HashMap<String, WorkflowDefinition>>,

    /// Thread-safe storage for resources
    /// Key: resource ID (Uuid), Value: resource
    resources: std::sync::RwLock<HashMap<Uuid, Resource>>,
}

/// Implementation of WorkflowStorage trait for in-memory storage
///
/// ## Rust Learning Notes:
///
/// ### Async Trait Implementation
/// The `#[async_trait::async_trait]` macro transforms async trait methods
/// into methods that return `Pin<Box<dyn Future<Output = T> + Send + '_>>`.
/// This enables async functions in traits.
///
/// ### Error Handling with ?
/// The `?` operator is used for error propagation:
/// - If Result is Ok(value), extract the value
/// - If Result is Err(error), return early with the error
/// - Much cleaner than explicit match statements
///
/// ### Clone vs Reference
/// We often call `.clone()` on data when returning it from storage.
/// This is because the storage owns the data, but we need to return
/// owned values to the caller. In a real database implementation,
/// this cloning would be replaced with deserialization.
#[async_trait::async_trait]
impl WorkflowStorage for InMemoryStorage {
    /// Create and store a workflow definition
    async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowDefinition> {
        // Get a write lock on the workflows HashMap
        // .unwrap() is used here because RwLock poisoning is rare in practice
        // In production code, you might want to handle poison errors explicitly
        let mut workflows = self.workflows.write().unwrap();

        // Store the workflow using its ID as the key
        workflows.insert(definition.id.clone(), definition.clone());

        // Return the workflow (could be modified by storage layer)
        Ok(definition)
    }

    /// Retrieve a workflow by ID
    async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDefinition>> {
        // Get a read lock - allows multiple concurrent readers
        let workflows = self.workflows.read().unwrap();

        // Look up the workflow and clone it if found
        // .cloned() is equivalent to .map(|w| w.clone())
        Ok(workflows.get(id).cloned())
    }

    /// List all stored workflows
    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        let workflows = self.workflows.read().unwrap();

        // Collect all values from the HashMap into a vector
        // .values() returns an iterator over the values
        // .cloned() clones each workflow definition
        // .collect() consumes the iterator and creates a Vec
        Ok(workflows.values().cloned().collect())
    }

    /// Create and store a new resource
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        let mut resources = self.resources.write().unwrap();

        // Store the resource using its UUID as the key
        resources.insert(resource.id, resource.clone());

        Ok(resource)
    }

    /// Retrieve a resource by UUID
    async fn get_resource(&self, id: &Uuid) -> Result<Option<Resource>> {
        let resources = self.resources.read().unwrap();

        // Look up resource by UUID and clone if found
        Ok(resources.get(id).cloned())
    }

    /// Update an existing resource (or create if it doesn't exist)
    async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        let mut resources = self.resources.write().unwrap();

        // Insert will either create or update the resource
        resources.insert(resource.id, resource.clone());

        Ok(resource)
    }

    /// List resources, optionally filtered by workflow ID
    async fn list_resources(&self, workflow_id: Option<&str>) -> Result<Vec<Resource>> {
        let resources = self.resources.read().unwrap();

        // Filter resources based on workflow_id parameter
        let filtered: Vec<Resource> = resources
            .values() // Get iterator over all resources
            .filter(|resource| {
                // Keep only resources that match filter
                // If workflow_id is None, keep all resources (map_or returns true)
                // If workflow_id is Some(id), keep only resources where workflow_id matches
                workflow_id.map_or(true, |wid| resource.workflow_id == wid)
            })
            .cloned() // Clone each resource
            .collect(); // Collect into vector

        Ok(filtered)
    }
}
