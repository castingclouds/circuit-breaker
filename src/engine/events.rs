// Event system for triggering functions

//! # Event System
//!
//! This module provides the event bus system that connects workflow operations
//! to function execution. It handles:
//! - Event emission from workflow operations
//! - Event subscription and routing
//! - Integration with function engine

use tokio::sync::broadcast;
use tracing::debug;
use uuid::Uuid;

use crate::models::{ActivityId, EventType, Resource, StateId, TriggerEvent};
use crate::Result;

/// Event bus for publishing and subscribing to workflow events
pub struct EventBus {
    sender: broadcast::Sender<TriggerEvent>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer up to 1000 events

        Self { sender }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: TriggerEvent) -> Result<()> {
        // Send to broadcast channel (for any future subscribers)
        let _ = self.sender.send(event.clone());

        // TODO: Process event with function engine when implemented
        debug!("Event published: {:?}", event.event_type);

        Ok(())
    }

    /// Subscribe to events (for future use)
    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.sender.subscribe()
    }

    /// Emit a resource created event
    pub async fn emit_resource_created(&self, resource: &Resource) -> Result<()> {
        let event = TriggerEvent::token_created(
            &resource.workflow_id,
            resource.id,
            StateId::from(resource.current_state()),
            resource.data.clone(),
            resource.metadata.clone(),
        );

        self.publish(event).await
    }

    /// Emit a resource transitioned event
    pub async fn emit_resource_transitioned(
        &self,
        resource: &Resource,
        from_state: StateId,
        activity: ActivityId,
    ) -> Result<()> {
        let event = TriggerEvent::token_transitioned(
            &resource.workflow_id,
            resource.id,
            from_state,
            StateId::from(resource.current_state()),
            activity,
            resource.data.clone(),
            resource.metadata.clone(),
        );

        self.publish(event).await
    }

    /// Emit a resource updated event
    pub async fn emit_resource_updated(&self, resource: &Resource) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::TokenUpdated {
                place: Some(StateId::from(resource.current_state())),
            },
            workflow_id: resource.workflow_id.clone(),
            token_id: Some(resource.id),
            data: resource.data.clone(),
            metadata: resource.metadata.clone(),
            timestamp: chrono::Utc::now(),
        };

        self.publish(event).await
    }

    /// Emit a workflow created event
    pub async fn emit_workflow_created(&self, workflow_id: &str) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::WorkflowCreated,
            workflow_id: workflow_id.to_string(),
            token_id: None,
            data: serde_json::Value::Null,
            metadata: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        self.publish(event).await
    }

    /// Emit a custom event
    pub async fn emit_custom_event(
        &self,
        event_name: String,
        workflow_id: String,
        token_id: Option<Uuid>,
        data: serde_json::Value,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::Custom { event_name },
            workflow_id,
            token_id,
            data,
            metadata,
            timestamp: chrono::Utc::now(),
        };

        self.publish(event).await
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// Extension trait to add event emission to Token operations
/// Extension trait for Resource to enable event-driven operations
pub trait ResourceEvents {
    /// Create a resource and emit creation event
    async fn new_with_events(
        workflow_id: &str,
        initial_state: StateId,
        event_bus: &EventBus,
    ) -> Result<Resource>;

    /// Execute activity to a new state and emit transition event
    async fn execute_activity_with_events(
        &mut self,
        new_state: StateId,
        activity_id: ActivityId,
        event_bus: &EventBus,
    ) -> Result<()>;

    /// Set metadata and emit update event
    async fn set_metadata_with_events<K: Into<String>>(
        &mut self,
        key: K,
        value: serde_json::Value,
        event_bus: &EventBus,
    ) -> Result<()>;
}

impl ResourceEvents for Resource {
    async fn new_with_events(
        workflow_id: &str,
        initial_state: StateId,
        event_bus: &EventBus,
    ) -> Result<Resource> {
        let resource = Resource::new(workflow_id, initial_state);
        event_bus.emit_resource_created(&resource).await?;
        Ok(resource)
    }

    async fn execute_activity_with_events(
        &mut self,
        new_state: StateId,
        activity_id: ActivityId,
        event_bus: &EventBus,
    ) -> Result<()> {
        let old_state = StateId::from(self.current_state());
        self.execute_activity(new_state, activity_id.clone());
        event_bus
            .emit_resource_transitioned(self, old_state, activity_id)
            .await?;
        Ok(())
    }

    async fn set_metadata_with_events<K: Into<String>>(
        &mut self,
        key: K,
        value: serde_json::Value,
        event_bus: &EventBus,
    ) -> Result<()> {
        self.metadata.insert(key.into(), value);
        event_bus.emit_resource_updated(self).await?;
        Ok(())
    }
}
